use axum::{
    Form,
    body::Body,
    extract::{Path, Query},
    http::{StatusCode, Uri, header},
    response::{IntoResponse, Redirect, Response},
};
use chrono::Utc;
use lariv_core::{
    components::{ButtonDownload, SharedChromeFolder, SlotCtx, button_download, modal_keyed},
    html_form::HtmlFormBody,
    http::Cap,
    template::RenderAppPane,
    web::{
        Htmx, ModalFormQuery, html_built_page_or_app_layout, html_built_page_with_slots,
        respond_create_modal_done, respond_edit_modal_done,
    },
};
use lariv_plugin_filesystem::{entities::VNodeEntity, state::FilesystemState};
use lariv_plugin_users::{
            middleware::{OptionalAuth, RequireAuth},
            roles::Superuser,
        };
use maud::{Markup, html};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter, QueryOrder,
};
use serde::Deserialize;
use std::collections::HashMap;

use super::{
    challan_number,
    entities::{
        DeliveryPreferences,
        delivery_challan::{self, Entity as ChallanEntity},
        delivery_challan_line::{self, Entity as LineEntity},
    },
    forms::DeliveryPreferencesForm,
    keys::*,
    pdf::{self, PdfError, PdfResult},
    pdf_templates::is_stock_delivery_challan_template,
    preferences::{
        delivery_challan_pdf_template, empty_preferences, load_preferences, opt_text, opt_vnode_id,
        save_preferences,
    },
    qty::{self, ParsedQty, QtyInput},
    routes::*,
    state::DeliveryState,
    templates::*,
};

fn delivery_staff(ctx: &lariv_plugin_users::state::AuthContext) -> bool {
    Superuser::matches(&ctx.role) || ctx.role == kds_plugin_accountant_role::ACCOUNTANT_ROLE
}

fn preferences_staff(ctx: &lariv_plugin_users::state::AuthContext) -> bool {
    Superuser::matches(&ctx.role)
}

fn path_and_query(uri: &Uri) -> String {
    uri.path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string())
}

#[derive(Debug, Deserialize)]
pub struct ChallanFormData {
    #[serde(default, alias = "ChallanNumber")]
    pub challan_number: String,
    #[serde(default, alias = "Date")]
    pub date: String,
    #[serde(default, alias = "CustomerID", alias = "customer_id")]
    pub customer_id: String,
    #[serde(default, alias = "VehicleNo")]
    pub vehicle_no: String,
    #[serde(default, alias = "EwayBill")]
    pub eway_bill: String,
    #[serde(default, alias = "Lines")]
    pub lines: String,
}

struct ParsedLine {
    sr_no: i32,
    product_id: i64,
    qty: ParsedQty,
}

fn parse_customer_id(raw: &str) -> i64 {
    raw.trim().parse().unwrap_or(0)
}

fn line_is_blank(line: &LineJson) -> bool {
    line.product_id <= 0
        && line.qty_mm.trim().is_empty()
        && line.qty_weight.trim().is_empty()
        && line.qty_number.trim().is_empty()
}

#[derive(Debug, Deserialize)]
struct LineJson {
    #[serde(default)]
    sr_no: i32,
    #[serde(default)]
    product_id: i64,
    #[serde(default)]
    qty_kind: String,
    #[serde(default)]
    qty_mm: String,
    #[serde(default)]
    qty_unit: String,
    #[serde(default)]
    qty_weight: String,
    #[serde(default)]
    qty_number: String,
}

fn parse_lines_json(raw: &str) -> Result<Vec<ParsedLine>, String> {
    let raw = raw.trim();
    if raw.is_empty() || raw == "[]" {
        return Ok(Vec::new());
    }
    let rows: Vec<LineJson> =
        serde_json::from_str(raw).map_err(|_| "Could not read lines.".to_string())?;
    let mut out = Vec::new();
    for (i, line) in rows.into_iter().enumerate() {
        if line_is_blank(&line) {
            continue;
        }
        if line.product_id <= 0 {
            return Err(format!("Select a product on line {}.", i + 1));
        }
        let qty = qty::parse_qty(&QtyInput {
            kind: line.qty_kind,
            qty_mm: line.qty_mm,
            qty_unit: line.qty_unit,
            qty_weight: line.qty_weight,
            qty_number: line.qty_number,
        })
        .map_err(|e| format!("Line {}: {e}", i + 1))?;
        let sr_no = if line.sr_no > 0 {
            line.sr_no
        } else {
            (i as i32) + 1
        };
        out.push(ParsedLine {
            sr_no,
            product_id: line.product_id,
            qty,
        });
    }
    Ok(out)
}

async fn replace_lines(
    db: &sea_orm::DatabaseConnection,
    challan_id: i64,
    lines: &[ParsedLine],
    now: chrono::DateTime<Utc>,
) -> Result<(), sea_orm::DbErr> {
    LineEntity::delete_many()
        .filter(delivery_challan_line::Column::DeliveryChallanId.eq(challan_id))
        .exec(db)
        .await?;
    for line in lines {
        let model = delivery_challan_line::ActiveModel {
            id: Default::default(),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            delivery_challan_id: Set(challan_id),
            sr_no: Set(line.sr_no),
            product_id: Set(line.product_id),
            qty_kind: Set(line.qty.kind.as_str().to_string()),
            qty_length: Set(line.qty.length_mm),
            qty_length_unit: Set(line.qty.length_unit.clone()),
            qty_weight: Set(line.qty.weight_kg),
            qty_number: Set(line.qty.number),
        };
        model.insert(db).await?;
    }
    Ok(())
}

fn dec_json(d: Option<Decimal>) -> String {
    d.map(|n| n.normalize().to_string()).unwrap_or_default()
}

async fn lines_editor_json(db: &sea_orm::DatabaseConnection, challan_id: i64) -> String {
    let lines = LineEntity::find()
        .filter(delivery_challan_line::Column::DeliveryChallanId.eq(challan_id))
        .order_by_asc(delivery_challan_line::Column::SrNo)
        .order_by_asc(delivery_challan_line::Column::Id)
        .all(db)
        .await
        .unwrap_or_default();
    let names = product_names(db, &lines).await;
    let rows: Vec<serde_json::Value> = lines
        .iter()
        .map(|line| {
            let label = names.get(&line.product_id).cloned().unwrap_or_default();
            serde_json::json!({
                "sr_no": line.sr_no,
                "product_id": line.product_id,
                "product_label": label,
                "qty_kind": line.qty_kind,
                "qty_mm": dec_json(line.qty_length),
                "qty_unit": line.qty_length_unit.clone().unwrap_or_else(|| "mm".into()),
                "qty_weight": dec_json(line.qty_weight),
                "qty_number": line.qty_number.map(|n| n.to_string()).unwrap_or_default(),
            })
        })
        .collect();
    serde_json::to_string(&rows).unwrap_or_else(|_| "[]".into())
}

async fn product_names(
    db: &sea_orm::DatabaseConnection,
    lines: &[delivery_challan_line::Model],
) -> HashMap<i64, String> {
    let ids: Vec<i64> = lines.iter().map(|l| l.product_id).collect();
    if ids.is_empty() {
        return HashMap::new();
    }
    lariv_plugin_finance_products::entities::product::Entity::find()
        .filter(lariv_plugin_finance_products::entities::product::Column::Id.is_in(ids))
        .all(db)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|p| (p.id, p.name))
        .collect()
}

async fn customer_name(db: &sea_orm::DatabaseConnection, id: i64) -> String {
    if id <= 0 {
        return String::new();
    }
    lariv_plugin_customer::entities::customer::Entity::find_by_id(id)
        .one(db)
        .await
        .ok()
        .flatten()
        .map(|c| c.name)
        .unwrap_or_else(|| format!("Customer #{id}"))
}

async fn load_lines(
    db: &sea_orm::DatabaseConnection,
    challan_id: i64,
) -> Vec<delivery_challan_line::Model> {
    LineEntity::find()
        .filter(delivery_challan_line::Column::DeliveryChallanId.eq(challan_id))
        .order_by_asc(delivery_challan_line::Column::SrNo)
        .order_by_asc(delivery_challan_line::Column::Id)
        .all(db)
        .await
        .unwrap_or_default()
}

pub async fn challan_list(
    Cap(state): Cap<DeliveryState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    htmx: Htmx,
    uri: Uri,
) -> Markup {
    let challans = ChallanEntity::find()
        .order_by_desc(delivery_challan::Column::Date)
        .order_by_desc(delivery_challan::Column::Id)
        .all(&state.db)
        .await
        .unwrap_or_default();
    let mut customer_names = Vec::with_capacity(challans.len());
    for challan in &challans {
        customer_names.push(customer_name(&state.db, challan.customer_id).await);
    }
    let page = ChallanListPage {
        challans,
        customer_names,
        path_and_query: path_and_query(&uri),
    };
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    if htmx.targets::<DeliveryChallanTableKey>() {
        return page.render_table();
    }
    if htmx.wants_main_content() {
        return page.render_main().into();
    }
    if htmx.wants_app_layout() {
        return page.render_pane().into();
    }
    html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx)
}

pub async fn challan_detail(
    Cap(state): Cap<DeliveryState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(challan) = ChallanEntity::find_by_id(id)
        .one(&state.db)
        .await
        .unwrap_or(None)
    else {
        return Redirect::to(&DeliveryDefaultRouteTag.url()).into_response();
    };
    let lines = load_lines(&state.db, challan.id).await;
    let names = product_names(&state.db, &lines).await;
    let line_rows: Vec<(delivery_challan_line::Model, String)> = lines
        .into_iter()
        .map(|line| {
            let name = names
                .get(&line.product_id)
                .cloned()
                .unwrap_or_else(|| format!("Product #{}", line.product_id));
            (line, name)
        })
        .collect();
    let customer = customer_name(&state.db, challan.customer_id).await;
    let page = ChallanDetailPage {
        challan,
        customer_name: customer,
        lines: line_rows,
    };
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx).into_response()
}

async fn create_error_modal(
    db: &sea_orm::DatabaseConnection,
    form_name: String,
    form: &ChallanFormData,
    error: String,
) -> ChallanCreateModalPage {
    let customer_id = parse_customer_id(&form.customer_id);
    ChallanCreateModalPage {
        form_name,
        challan_number: form.challan_number.clone(),
        date: form.date.clone(),
        vehicle_no: form.vehicle_no.clone(),
        eway_bill: form.eway_bill.clone(),
        customer_id: (customer_id > 0).then_some(customer_id),
        customer_name: customer_name(db, customer_id).await,
        lines_json: form.lines.clone(),
        error,
    }
}

async fn edit_error_modal(
    db: &sea_orm::DatabaseConnection,
    id: i64,
    form_name: String,
    form: &ChallanFormData,
    error: String,
) -> ChallanEditModalPage {
    let customer_id = parse_customer_id(&form.customer_id);
    ChallanEditModalPage {
        id,
        form_name,
        challan_number: form.challan_number.clone(),
        date: form.date.clone(),
        vehicle_no: form.vehicle_no.clone(),
        eway_bill: form.eway_bill.clone(),
        customer_id,
        customer_name: customer_name(db, customer_id).await,
        lines_json: form.lines.clone(),
        error,
    }
}

pub async fn challan_create_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    Query(q): Query<ModalFormQuery>,
) -> Markup {
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let page = ChallanCreateModalPage {
        form_name: q.form_name(),
        challan_number: String::new(),
        date: today,
        vehicle_no: String::new(),
        eway_bill: String::new(),
        customer_id: None,
        customer_name: String::new(),
        lines_json: "[]".into(),
        error: String::new(),
    };
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    html_built_page_with_slots(&page, &chrome, &slot_ctx)
}

pub async fn challan_create_post(
    Cap(state): Cap<DeliveryState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    htmx: Htmx,
    Query(q): Query<ModalFormQuery>,
    Form(form): Form<ChallanFormData>,
) -> Response {
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    let customer_id = parse_customer_id(&form.customer_id);
    if customer_id <= 0 {
        let page = create_error_modal(
            &state.db,
            q.form_name(),
            &form,
            "Please select a customer.".into(),
        )
        .await;
        return html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response();
    }
    let date = match chrono::NaiveDate::parse_from_str(form.date.trim(), "%Y-%m-%d") {
        Ok(d) => d,
        Err(_) => {
            let page = create_error_modal(
                &state.db,
                q.form_name(),
                &form,
                "Invalid date format. Use YYYY-MM-DD.".into(),
            )
            .await;
            return html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response();
        }
    };
    let lines = match parse_lines_json(&form.lines) {
        Ok(lines) => lines,
        Err(e) => {
            let page = create_error_modal(&state.db, q.form_name(), &form, e).await;
            return html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response();
        }
    };
    let challan_number =
        match challan_number::resolve_challan_number(&state.db, &form.challan_number, date).await {
            Ok(n) => n,
            Err(e) => {
                let page = create_error_modal(
                    &state.db,
                    q.form_name(),
                    &form,
                    format!("Failed to assign challan number: {e}"),
                )
                .await;
                return html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response();
            }
        };
    let now = Utc::now();
    let model = delivery_challan::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        date: Set(date),
        customer_id: Set(customer_id),
        challan_number: Set(challan_number),
        vehicle_no: Set(opt_text(&form.vehicle_no)),
        eway_bill: Set(opt_text(&form.eway_bill)),
    };
    match model.insert(&state.db).await {
        Ok(saved) => {
            if let Err(e) = replace_lines(&state.db, saved.id, &lines, now).await {
                let _ = ChallanEntity::delete_by_id(saved.id).exec(&state.db).await;
                let page = create_error_modal(&state.db, q.form_name(), &form, e.to_string()).await;
                return html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response();
            }
            respond_create_modal_done::<DeliveryChallanCreateModalKey>(
                &htmx,
                &q.refresh_table(),
                &DeliveryChallanDetailRouteTag::new(saved.id).url(),
            )
        }
        Err(e) => {
            let page = create_error_modal(&state.db, q.form_name(), &form, e.to_string()).await;
            html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response()
        }
    }
}

pub async fn challan_edit_get(
    Cap(state): Cap<DeliveryState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    Query(q): Query<ModalFormQuery>,
    Path(id): Path<i64>,
) -> Response {
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    let challan = match ChallanEntity::find_by_id(id).one(&state.db).await {
        Ok(Some(row)) => row,
        _ => return Redirect::to(&DeliveryDefaultRouteTag.url()).into_response(),
    };
    let page = ChallanEditModalPage {
        id,
        form_name: q.form_name(),
        challan_number: challan.challan_number,
        date: challan.date.to_string(),
        vehicle_no: challan.vehicle_no.unwrap_or_default(),
        eway_bill: challan.eway_bill.unwrap_or_default(),
        customer_id: challan.customer_id,
        customer_name: customer_name(&state.db, challan.customer_id).await,
        lines_json: lines_editor_json(&state.db, id).await,
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response()
}

pub async fn challan_edit_post(
    Cap(state): Cap<DeliveryState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    htmx: Htmx,
    Query(q): Query<ModalFormQuery>,
    Path(id): Path<i64>,
    Form(form): Form<ChallanFormData>,
) -> Response {
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    let existing = match ChallanEntity::find_by_id(id).one(&state.db).await {
        Ok(Some(row)) => row,
        _ => return Redirect::to(&DeliveryDefaultRouteTag.url()).into_response(),
    };
    let customer_id = parse_customer_id(&form.customer_id);
    if customer_id <= 0 {
        let page = edit_error_modal(
            &state.db,
            id,
            q.form_name(),
            &form,
            "Please select a customer.".into(),
        )
        .await;
        return html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response();
    }
    let date = match chrono::NaiveDate::parse_from_str(form.date.trim(), "%Y-%m-%d") {
        Ok(d) => d,
        Err(_) => existing.date,
    };
    let lines = match parse_lines_json(&form.lines) {
        Ok(lines) => lines,
        Err(e) => {
            let page = edit_error_modal(&state.db, id, q.form_name(), &form, e).await;
            return html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response();
        }
    };
    let challan_number =
        match challan_number::resolve_challan_number(&state.db, &form.challan_number, date).await {
            Ok(n) => n,
            Err(e) => {
                let page = edit_error_modal(
                    &state.db,
                    id,
                    q.form_name(),
                    &form,
                    format!("Failed to assign challan number: {e}"),
                )
                .await;
                return html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response();
            }
        };
    let now = Utc::now();
    let mut am: delivery_challan::ActiveModel = existing.into();
    am.updated_at = Set(Some(now));
    am.challan_number = Set(challan_number);
    am.date = Set(date);
    am.customer_id = Set(customer_id);
    am.vehicle_no = Set(opt_text(&form.vehicle_no));
    am.eway_bill = Set(opt_text(&form.eway_bill));
    match am.update(&state.db).await {
        Ok(_) => {
            if let Err(e) = replace_lines(&state.db, id, &lines, now).await {
                let page =
                    edit_error_modal(&state.db, id, q.form_name(), &form, e.to_string()).await;
                return html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response();
            }
            respond_edit_modal_done::<DeliveryChallanEditModalKey>(
                &htmx,
                &DeliveryChallanDetailRouteTag::new(id).url(),
            )
        }
        Err(e) => {
            let page = edit_error_modal(&state.db, id, q.form_name(), &form, e.to_string()).await;
            html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response()
        }
    }
}

pub async fn challan_delete_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    Path(id): Path<i64>,
) -> Markup {
    let page = ConfirmDeleteModalPage {
        title: "Delete Delivery Challan".into(),
        message: "Are you sure you want to delete this delivery challan?".into(),
        post_url: DeliveryChallanDeletePostRouteTag::new(id).url(),
        error: String::new(),
    };
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    html_built_page_with_slots(&page, &chrome, &slot_ctx)
}

pub async fn challan_delete_post(
    Cap(state): Cap<DeliveryState>,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let _ = ChallanEntity::delete_by_id(id).exec(&state.db).await;
    htmx.redirect(&DeliveryDefaultRouteTag.url())
}

fn fk_value(id: Option<i64>) -> String {
    id.filter(|&id| id > 0).unwrap_or(0).to_string()
}

async fn load_vnode_display(db: &sea_orm::DatabaseConnection, id: Option<i64>) -> String {
    let Some(id) = id.filter(|&id| id > 0) else {
        return String::new();
    };
    VNodeEntity::find_by_id(id)
        .one(db)
        .await
        .ok()
        .flatten()
        .map(|n| n.name)
        .unwrap_or_default()
}

async fn prefs_page(
    db: &sea_orm::DatabaseConnection,
    prefs: DeliveryPreferences,
    error: String,
) -> DeliveryPreferencesPage {
    let template = delivery_challan_pdf_template(&prefs).to_string();
    let logo_id = prefs.logo_vnode_id;
    let signature_id = prefs.signature_vnode_id;
    DeliveryPreferencesPage {
        challan_number_format: prefs.challan_number_format.unwrap_or_default(),
        company_name: prefs.company_name.unwrap_or_default(),
        company_address: prefs.company_address.unwrap_or_default(),
        company_phone: prefs.company_phone.unwrap_or_default(),
        company_email: prefs.company_email.unwrap_or_default(),
        company_gstin: prefs.company_gstin.unwrap_or_default(),
        terms_and_conditions: prefs.terms_and_conditions.unwrap_or_default(),
        logo_vnode_id: fk_value(logo_id),
        logo_vnode_display: load_vnode_display(db, logo_id).await,
        signature_vnode_id: fk_value(signature_id),
        signature_vnode_display: load_vnode_display(db, signature_id).await,
        delivery_challan_pdf_template: template,
        error,
    }
}

pub async fn preferences_get(
    Cap(state): Cap<DeliveryState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
) -> Response {
    if !preferences_staff(&ctx) {
        return Redirect::to(&DeliveryDefaultRouteTag.url()).into_response();
    }
    let slot_ctx = SlotCtx::from_auth(&ctx);
    let prefs = match load_preferences(&state.db).await {
        Ok(p) => p,
        Err(e) => {
            let page = prefs_page(
                &state.db,
                empty_preferences(),
                format!("Failed to load preferences: {e}"),
            )
            .await;
            return html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx).into_response();
        }
    };
    let page = prefs_page(&state.db, prefs, String::new()).await;
    html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx).into_response()
}

pub async fn preferences_post(
    Cap(state): Cap<DeliveryState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    HtmlFormBody(form): HtmlFormBody<DeliveryPreferencesForm>,
) -> Response {
    if !Superuser::matches(&ctx.role) {
        return Redirect::to(&DeliveryDefaultRouteTag.url()).into_response();
    }
    let slot_ctx = SlotCtx::from_auth(&ctx);
    let challan_number_format = {
        let t = form.challan_number_format.trim();
        if t.is_empty() {
            None
        } else {
            Some(t.to_string())
        }
    };
    let prefs = DeliveryPreferences {
        id: 1,
        created_at: None,
        updated_at: None,
        company_name: opt_text(&form.company_name),
        company_address: opt_text(&form.company_address),
        company_phone: opt_text(&form.company_phone),
        company_email: opt_text(&form.company_email),
        company_gstin: opt_text(&form.company_gstin),
        terms_and_conditions: opt_text(&form.terms_and_conditions),
        challan_number_format,
        delivery_challan_pdf_template: if is_stock_delivery_challan_template(Some(
            form.delivery_challan_pdf_template.as_str(),
        )) {
            None
        } else {
            Some(form.delivery_challan_pdf_template)
        },
        logo_vnode_id: opt_vnode_id(&form.logo_vnode_id),
        signature_vnode_id: opt_vnode_id(&form.signature_vnode_id),
    };
    match save_preferences(&state.db, prefs.clone()).await {
        Ok(_) => htmx.redirect(&DeliveryPrefsGetRouteTag.url()),
        Err(e) => {
            let page =
                prefs_page(&state.db, prefs, format!("Failed to save preferences: {e}")).await;
            html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx).into_response()
        }
    }
}

pub async fn challan_pdf_modal(
    Cap(state): Cap<DeliveryState>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
) -> Markup {
    if !delivery_staff(&ctx) {
        return render_pdf_modal_error("Forbidden");
    }
    let Some(challan) = ChallanEntity::find_by_id(id)
        .one(&state.db)
        .await
        .unwrap_or(None)
    else {
        return render_pdf_modal_error("Delivery challan not found");
    };
    let title = if challan.challan_number.trim().is_empty() {
        "Delivery Challan PDF".to_string()
    } else {
        format!("Delivery Challan {} PDF", challan.challan_number)
    };
    render_pdf_modal(&title, &DeliveryChallanPdfRouteTag::new(id).path())
}

pub async fn challan_pdf(
    Cap(state): Cap<DeliveryState>,
    Cap(fs): Cap<FilesystemState>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
) -> Response {
    if !delivery_staff(&ctx) {
        return StatusCode::FORBIDDEN.into_response();
    }
    match pdf::render_challan_pdf(&state.db, Some(&fs), id).await {
        Ok(result) => pdf_ok_response(result),
        Err(e) => pdf_error_response(e),
    }
}

fn pdf_error_response(err: PdfError) -> Response {
    match err {
        PdfError::NotFound => (StatusCode::NOT_FOUND, "Not found").into_response(),
        PdfError::Message(msg) => {
            tracing::error!("delivery challan pdf: {msg}");
            (StatusCode::INTERNAL_SERVER_ERROR, msg).into_response()
        }
    }
}

fn pdf_ok_response(result: PdfResult) -> Response {
    let filename = format!("{}.pdf", result.filename_base);
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/pdf".to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("inline; filename=\"{filename}\""),
            ),
        ],
        Body::from(result.bytes),
    )
        .into_response()
}

fn render_pdf_modal(title: &str, pdf_url: &str) -> Markup {
    modal_keyed::<DeliveryPdfModalKey>(
        "max-w-6xl w-[95vw]",
        html! {
            div class="flex items-center justify-between gap-3 mb-3 pr-10" {
                h3 class="text-lg font-semibold" { (title) }
                (button_download(ButtonDownload {
                    label: "Download",
                    href: pdf_url,
                    classes: "btn-outline btn-sm",
                    ..Default::default()
                }))
            }
            div class="relative w-full h-[75vh]" x-data="{ loading: true }" {
                div
                    class="absolute inset-0 z-10 flex flex-col items-center justify-center gap-3 rounded border border-base-300 bg-base-100"
                    x-show="loading"
                    x-cloak
                {
                    span class="loading loading-spinner loading-lg" {}
                    p class="text-sm opacity-70" { "Generating PDF…" }
                }
                iframe
                    src=(pdf_url)
                    class="w-full h-full border border-base-300 rounded bg-white"
                    title=(title)
                    x-on:load="loading = false" {}
            }
        },
    )
}

fn render_pdf_modal_error(message: &str) -> Markup {
    modal_keyed::<DeliveryPdfModalKey>(
        "max-w-2xl",
        html! {
            h3 class="text-lg font-semibold mb-2" { "PDF preview failed" }
            p class="text-error whitespace-pre-wrap" { (message) }
        },
    )
}
