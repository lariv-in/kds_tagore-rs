use axum::{
    Form, Json,
    body::Body,
    extract::{Path, Query},
    http::{StatusCode, Uri, header},
    response::{IntoResponse, Redirect, Response},
};
use chrono::Utc;
use lariv_rs::{
    components::{
        ButtonDownload, ManyToManyItem, SharedChromeFolder, SlotCtx, SwapKey, button_download,
        modal_keyed,
    },
    html_form::HtmlFormBody,
    http::Cap,
    picker::respond_picker_select,
    plugins::{
        filesystem::{entities::VNodeEntity, state::FilesystemState},
        finance_common::require_superuser,
        users::middleware::{OptionalAuth, RequireAuth, RequireStaff},
    },
    template::RenderAppPane,
    web::{
        Htmx, ModalFormQuery, html_built_page_or_app_layout, html_built_page_with_slots,
        respond_create_modal_done, respond_create_modal_done_fk, respond_edit_modal_done,
    },
};
use maud::{Markup, html};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter,
    QueryOrder, Statement, TransactionTrait,
};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

use super::{
    entities::{
        WorkOrdersPreferences, component, draft_work_order, draft_work_order_machine_line,
        draft_work_order_material_line, quotation, quotation_machine_line, quotation_material_line,
        work_order, work_order_line, work_order_machine_line,
    },
    forms::WorkOrdersPreferencesForm,
    keys::*,
    line_vars,
    pdf::{self, PdfError, PdfResult},
    pdf_templates::{
        is_stock_draft_work_order_template, is_stock_quotation_template,
        is_stock_work_order_template,
    },
    preferences::{
        draft_work_order_pdf_template, empty_preferences, load_preferences, opt_text, opt_vnode_id,
        quotation_pdf_template, save_preferences, work_order_pdf_template,
    },
    quotation_number,
    routes::*,
    state::WorkOrdersState,
    tax_assoc,
    templates::*,
};

use crate::formula::{parse_schema_list, parse_values_from_json, schema_to_json, values_to_json};
use crate::machinery_schedule::duration::JobDuration;
use crate::machinery_schedule::entities::{job, machine};
use crate::machinery_schedule::logic::{
    create_open_job, format_job_duration, parse_job_duration, set_job_source_doc,
};
use crate::work_orders::cascade::{
    cascade_delete_preview, collect_work_order_cascade, delete_work_order_recursive,
};
use crate::work_orders::entities::work_order::WORK_ORDER_SOURCE_DOC_TYPE;

fn path_and_query(uri: &Uri) -> String {
    uri.path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string())
}

// ==========================================
// 1. WORK ORDERS
// ==========================================

pub async fn work_orders_list(
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    htmx: Htmx,
    uri: Uri,
) -> maud::Markup {
    let orders = draft_work_order::Entity::find()
        .order_by_desc(draft_work_order::Column::Id)
        .all(&state.db)
        .await
        .unwrap_or_default();

    let all_lines = draft_work_order_material_line::Entity::find()
        .all(&state.db)
        .await
        .unwrap_or_default();

    let all_machine_lines = draft_work_order_machine_line::Entity::find()
        .all(&state.db)
        .await
        .unwrap_or_default();

    let mut lines_by_order: HashMap<i64, Vec<draft_work_order_material_line::Model>> =
        HashMap::new();
    for line in all_lines {
        lines_by_order
            .entry(line.draft_work_order_id)
            .or_default()
            .push(line);
    }

    let mut machine_lines_by_order: HashMap<i64, Vec<draft_work_order_machine_line::Model>> =
        HashMap::new();
    for line in all_machine_lines {
        machine_lines_by_order
            .entry(line.draft_work_order_id)
            .or_default()
            .push(line);
    }

    let material_ids: Vec<i64> = lines_by_order.values().flatten().map(|l| l.id).collect();
    let machine_ids: Vec<i64> = machine_lines_by_order
        .values()
        .flatten()
        .map(|l| l.id)
        .collect();
    let material_tax_ids =
        tax_assoc::load_draft_material_line_tax_ids_map(&state.db, &material_ids)
            .await
            .unwrap_or_default();
    let machine_tax_ids = tax_assoc::load_draft_machine_line_tax_ids_map(&state.db, &machine_ids)
        .await
        .unwrap_or_default();
    let material_taxes = tax_assoc::resolve_taxes_by_line_id(&state.db, &material_tax_ids).await;
    let machine_taxes = tax_assoc::resolve_taxes_by_line_id(&state.db, &machine_tax_ids).await;

    let orders_with_stats: Vec<(draft_work_order::Model, usize, Decimal)> = orders
        .into_iter()
        .map(|o| {
            let lines = lines_by_order
                .get(&o.id)
                .map(|v| v.as_slice())
                .unwrap_or(&[]);
            let machine_lines = machine_lines_by_order
                .get(&o.id)
                .map(|v| v.as_slice())
                .unwrap_or(&[]);
            let total =
                taxed_draft_grand_total(lines, machine_lines, &material_taxes, &machine_taxes);
            let count = lines.len() + machine_lines.len();
            (o, count, total)
        })
        .collect();

    let page = WorkOrderListPage {
        orders: orders_with_stats,
        path_and_query: path_and_query(&uri),
    };
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    if htmx.targets::<WorkOrderTableKey>() {
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

pub async fn work_order_detail(
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(order) = draft_work_order::Entity::find_by_id(id)
        .one(&state.db)
        .await
        .unwrap_or(None)
    else {
        return Redirect::to(&DraftWorkOrdersDefaultRouteTag.url()).into_response();
    };

    let lines = draft_work_order_material_line::Entity::find()
        .filter(draft_work_order_material_line::Column::DraftWorkOrderId.eq(id))
        .order_by_asc(draft_work_order_material_line::Column::Id)
        .all(&state.db)
        .await
        .unwrap_or_default();

    let machine_lines = draft_work_order_machine_line::Entity::find()
        .filter(draft_work_order_machine_line::Column::DraftWorkOrderId.eq(id))
        .order_by_asc(draft_work_order_machine_line::Column::Id)
        .all(&state.db)
        .await
        .unwrap_or_default();

    let machine_ids: Vec<i64> = machine_lines.iter().map(|l| l.machine_id).collect();
    let machines = machine::Entity::find()
        .filter(machine::Column::Id.is_in(machine_ids))
        .all(&state.db)
        .await
        .unwrap_or_default();
    let machine_map: HashMap<i64, String> =
        machines.iter().map(|m| (m.id, m.name.clone())).collect();
    let machine_schemas: HashMap<i64, serde_json::Value> =
        machines.into_iter().map(|m| (m.id, m.variables)).collect();

    let comp_ids: Vec<i64> = lines.iter().map(|l| l.component_id).collect();
    let comps = component::Entity::find()
        .filter(component::Column::Id.is_in(comp_ids))
        .all(&state.db)
        .await
        .unwrap_or_default();
    let comp_map: HashMap<i64, String> = comps.iter().map(|c| (c.id, c.name.clone())).collect();
    let component_schemas: HashMap<i64, serde_json::Value> =
        comps.into_iter().map(|c| (c.id, c.variables)).collect();

    let customer =
        lariv_rs::plugins::customer::entities::customer::Entity::find_by_id(order.customer_id)
            .one(&state.db)
            .await
            .ok()
            .flatten();

    let quotation_number = if let Some(qid) = order.quotation_id {
        quotation::Entity::find_by_id(qid)
            .one(&state.db)
            .await
            .ok()
            .flatten()
            .map(|q| q.invoice_number)
    } else {
        None
    };

    let material_ids: Vec<i64> = lines.iter().map(|l| l.id).collect();
    let machine_line_ids: Vec<i64> = machine_lines.iter().map(|l| l.id).collect();
    let material_tax_ids =
        tax_assoc::load_draft_material_line_tax_ids_map(&state.db, &material_ids)
            .await
            .unwrap_or_default();
    let machine_tax_ids =
        tax_assoc::load_draft_machine_line_tax_ids_map(&state.db, &machine_line_ids)
            .await
            .unwrap_or_default();
    let material_taxes = tax_assoc::resolve_taxes_by_line_id(&state.db, &material_tax_ids).await;
    let machine_taxes = tax_assoc::resolve_taxes_by_line_id(&state.db, &machine_tax_ids).await;

    let total_amount =
        taxed_draft_grand_total(&lines, &machine_lines, &material_taxes, &machine_taxes);

    let lines_with_comp: Vec<(
        draft_work_order_material_line::Model,
        String,
        String,
        Decimal,
    )> = lines
        .into_iter()
        .map(|l| {
            let c_name = comp_map
                .get(&l.component_id)
                .cloned()
                .unwrap_or_else(|| format!("Component #{}", l.component_id));
            let taxes = tax_assoc::taxes_for_line(&material_taxes, l.id);
            let labels = tax_assoc::tax_labels_display(taxes);
            let taxed = l.taxed_total(taxes);
            (l, c_name, labels, taxed)
        })
        .collect();

    let machine_lines_with_name: Vec<(
        draft_work_order_machine_line::Model,
        String,
        String,
        Decimal,
    )> = machine_lines
        .into_iter()
        .map(|l| {
            let m_name = machine_map
                .get(&l.machine_id)
                .cloned()
                .unwrap_or_else(|| format!("Machine #{}", l.machine_id));
            let taxes = tax_assoc::taxes_for_line(&machine_taxes, l.id);
            let labels = tax_assoc::tax_labels_display(taxes);
            let taxed = l.taxed_total(taxes);
            (l, m_name, labels, taxed)
        })
        .collect();

    let page = WorkOrderDetailPage {
        order,
        lines: lines_with_comp,
        machine_lines: machine_lines_with_name,
        component_schemas,
        machine_schemas,
        customer_name: customer.map(|c| c.name),
        quotation_number,
        total_amount,
    };
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx).into_response()
}

#[derive(Debug, Deserialize, Default)]
pub struct EntitySelectQuery {
    #[serde(default)]
    pub target_input: Option<String>,
    #[serde(default, rename = "Name", alias = "name")]
    pub name: Option<String>,
}

pub async fn work_order_select(
    Cap(state): Cap<WorkOrdersState>,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<EntitySelectQuery>,
) -> maud::Markup {
    let mut query = draft_work_order::Entity::find();
    if let Some(n) = q.name.as_deref().filter(|s| !s.trim().is_empty()) {
        query = query.filter(draft_work_order::Column::OrderNumber.contains(n));
    }
    let orders = query
        .order_by_desc(draft_work_order::Column::Id)
        .all(&state.db)
        .await
        .unwrap_or_default();
    let page = WorkOrderSelectPage {
        orders,
        target_input: q.target_input.unwrap_or_else(|| "work_order_id".into()),
        path_and_query: uri.to_string(),
    };
    respond_picker_select::<WorkOrderSelectTableKey, WorkOrderSelectModalKey, _>(&htmx, &page)
}

pub async fn fetch_components_meta(
    db: &sea_orm::DatabaseConnection,
) -> Vec<super::forms::ComponentMeta> {
    let comps = component::Entity::find()
        .order_by_asc(component::Column::Name)
        .all(db)
        .await
        .unwrap_or_default();

    comps
        .into_iter()
        .map(|c| {
            let variables = match &c.variables {
                serde_json::Value::Object(m) => m
                    .iter()
                    .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                    .collect(),
                _ => HashMap::new(),
            };
            super::forms::ComponentMeta {
                id: c.id,
                name: c.name,
                variables,
                cost_formula: c.cost_formula,
                weight_formula: c.weight_formula,
            }
        })
        .collect()
}

fn extra_data_from_value(val: Option<&serde_json::Value>) -> serde_json::Value {
    match val {
        Some(serde_json::Value::Object(_)) | Some(serde_json::Value::Array(_)) => {
            val.cloned().unwrap_or_else(|| serde_json::json!({}))
        }
        Some(serde_json::Value::String(s)) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                serde_json::json!({})
            } else {
                serde_json::from_str(trimmed).unwrap_or_else(|_| serde_json::json!({}))
            }
        }
        _ => serde_json::json!({}),
    }
}

fn variables_from_value(val: Option<&serde_json::Value>) -> serde_json::Value {
    match val {
        Some(v) => v.clone(),
        None => serde_json::json!({}),
    }
}

fn parse_optional_job_duration(raw: &str, fallback: JobDuration) -> Result<JobDuration, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        Ok(fallback)
    } else {
        parse_job_duration(trimmed)
    }
}

fn parse_component_variables_list(entries: &[String]) -> Result<serde_json::Value, String> {
    let schema = parse_schema_list(entries).map_err(|e| e.to_string())?;
    Ok(schema_to_json(&schema))
}

fn component_from_form_fields(
    name: String,
    cost_formula: String,
    weight_formula: String,
    variables: serde_json::Value,
) -> component::Model {
    component::Model {
        id: 0,
        created_at: None,
        updated_at: None,
        name,
        cost_formula,
        weight_formula,
        variables,
    }
}

async fn resolve_and_compute_line_data(
    db: &sea_orm::DatabaseConnection,
    component_id: i64,
    variables_val: Option<&serde_json::Value>,
    extra_data_val: Option<&serde_json::Value>,
) -> Result<(sea_orm::prelude::Json, Decimal, sea_orm::prelude::Json), String> {
    let comp = component::Entity::find_by_id(component_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Component #{} not found", component_id))?;

    let extra_data = extra_data_from_value(extra_data_val);
    let units = line_vars::dim_units_map(&extra_data);
    let schema = comp.variables_schema().map_err(|e| e.to_string())?;
    let values = parse_values_from_json(&schema, &variables_from_value(variables_val), &units)
        .map_err(|e| e.to_string())?;
    let final_cost = comp.get_cost(&values).map_err(|e| e.to_string())?;
    Ok((values_to_json(&values), final_cost, extra_data))
}

async fn resolve_and_compute_machine_cost(
    db: &sea_orm::DatabaseConnection,
    machine_id: i64,
    variables_val: Option<&serde_json::Value>,
) -> Result<(sea_orm::prelude::Json, Decimal, String), String> {
    let mach = machine::Entity::find_by_id(machine_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Machine #{} not found", machine_id))?;
    let schema = mach.variables_schema().map_err(|e| e.to_string())?;
    let values = parse_values_from_json(
        &schema,
        &variables_from_value(variables_val),
        &HashMap::new(),
    )
    .map_err(|e| e.to_string())?;
    let final_cost = mach.get_cost(&values).map_err(|e| e.to_string())?;
    Ok((values_to_json(&values), final_cost, mach.name))
}

pub async fn work_order_create_get(
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    Query(q): Query<ModalFormQuery>,
) -> maud::Markup {
    let components = fetch_components_meta(&state.db).await;
    let components_json = serde_json::to_string(&components).unwrap_or_else(|_| "[]".into());
    let machines_json = fetch_machines_json(&state.db).await;
    let page = WorkOrderCreateModalPage {
        form_name: q.form_name(),
        order_number: String::new(),
        customer_id: None,
        customer_name: String::new(),
        duration: String::new(),
        items_json: "[]".into(),
        components_json,
        machine_lines_json: "[]".into(),
        machines_json,
        taxes_json: tax_assoc::taxes_catalog_json(&state.db).await,
        error: String::new(),
    };
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    html_built_page_with_slots(&page, &chrome, &slot_ctx)
}

fn i64_from_str_or_zero<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let val = serde_json::Value::deserialize(deserializer)?;
    match val {
        serde_json::Value::Number(n) => Ok(n.as_i64().unwrap_or(0)),
        serde_json::Value::String(s) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                Ok(0)
            } else {
                trimmed.parse::<i64>().map_err(serde::de::Error::custom)
            }
        }
        serde_json::Value::Null => Ok(0),
        _ => Err(serde::de::Error::custom("expected integer or string")),
    }
}

fn opt_i64_from_str<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let val = serde_json::Value::deserialize(deserializer)?;
    match val {
        serde_json::Value::Number(n) => Ok(n.as_i64()),
        serde_json::Value::String(s) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                Ok(None)
            } else {
                trimmed
                    .parse::<i64>()
                    .map(Some)
                    .map_err(serde::de::Error::custom)
            }
        }
        serde_json::Value::Null => Ok(None),
        _ => Err(serde::de::Error::custom("expected integer or string")),
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct WorkOrderCreateForm {
    #[serde(alias = "order_number", default)]
    pub order_number: String,
    #[serde(
        alias = "CustomerID",
        alias = "customer_id",
        default,
        deserialize_with = "i64_from_str_or_zero"
    )]
    pub customer_id: i64,
    #[serde(alias = "items", default)]
    pub items: Option<String>,
    #[serde(alias = "machine_lines", default)]
    pub machine_lines: Option<String>,
    #[serde(alias = "duration", default)]
    pub duration: String,
}

pub async fn work_order_create_post(
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    htmx: Htmx,
    Query(q): Query<ModalFormQuery>,
    Form(form): Form<WorkOrderCreateForm>,
) -> Response {
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    let now = Utc::now();

    if form.customer_id <= 0 {
        let components = fetch_components_meta(&state.db).await;
        let components_json = serde_json::to_string(&components).unwrap_or_else(|_| "[]".into());
        let machines_json = fetch_machines_json(&state.db).await;
        let page = WorkOrderCreateModalPage {
            form_name: q.form_name(),
            order_number: form.order_number,
            customer_id: None,
            customer_name: String::new(),
            duration: form.duration.clone(),
            items_json: form.items.unwrap_or_default(),
            components_json,
            machine_lines_json: form.machine_lines.unwrap_or_default(),
            machines_json,
            taxes_json: tax_assoc::taxes_catalog_json(&state.db).await,
            error: "Please select a customer.".into(),
        };
        return html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response();
    }

    if let Err(e) = validate_machine_lines_json(form.machine_lines.as_deref()) {
        let components = fetch_components_meta(&state.db).await;
        let components_json = serde_json::to_string(&components).unwrap_or_else(|_| "[]".into());
        let machines_json = fetch_machines_json(&state.db).await;
        let customer_name =
            lariv_rs::plugins::customer::entities::customer::Entity::find_by_id(form.customer_id)
                .one(&state.db)
                .await
                .ok()
                .flatten()
                .map(|c| c.name)
                .unwrap_or_default();
        let page = WorkOrderCreateModalPage {
            form_name: q.form_name(),
            order_number: form.order_number,
            customer_id: Some(form.customer_id),
            customer_name,
            duration: form.duration.clone(),
            items_json: form.items.unwrap_or_default(),
            components_json,
            machine_lines_json: form.machine_lines.unwrap_or_default(),
            machines_json,
            taxes_json: tax_assoc::taxes_catalog_json(&state.db).await,
            error: e,
        };
        return html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response();
    }

    let duration = match parse_optional_job_duration(&form.duration, JobDuration::default()) {
        Ok(d) => d,
        Err(e) => {
            let components = fetch_components_meta(&state.db).await;
            let components_json =
                serde_json::to_string(&components).unwrap_or_else(|_| "[]".into());
            let machines_json = fetch_machines_json(&state.db).await;
            let customer_name =
                lariv_rs::plugins::customer::entities::customer::Entity::find_by_id(
                    form.customer_id,
                )
                .one(&state.db)
                .await
                .ok()
                .flatten()
                .map(|c| c.name)
                .unwrap_or_default();
            let page = WorkOrderCreateModalPage {
                form_name: q.form_name(),
                order_number: form.order_number,
                customer_id: Some(form.customer_id),
                customer_name,
                duration: form.duration.clone(),
                items_json: form.items.unwrap_or_default(),
                components_json,
                machine_lines_json: form.machine_lines.unwrap_or_default(),
                machines_json,
                taxes_json: tax_assoc::taxes_catalog_json(&state.db).await,
                error: format!("Invalid duration: {e}"),
            };
            return html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response();
        }
    };

    let model = draft_work_order::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        order_number: Set(form.order_number.trim().to_string()),
        customer_id: Set(form.customer_id),
        quotation_id: Set(None),
        duration: Set(duration),
    };

    match model.insert(&state.db).await {
        Ok(saved) => {
            if let Some(items_str) = form.items.as_deref().filter(|s| !s.trim().is_empty()) {
                if let Ok(items) =
                    serde_json::from_str::<Vec<super::forms::DraftWorkOrderLineInput>>(items_str)
                {
                    for item in items {
                        if item.component_id <= 0 {
                            continue;
                        }
                        if let Ok((vars_json, final_cost, extra_data)) =
                            resolve_and_compute_line_data(
                                &state.db,
                                item.component_id,
                                item.variables.as_ref(),
                                item.extra_data.as_ref(),
                            )
                            .await
                        {
                            let line_am = draft_work_order_material_line::ActiveModel {
                                id: Default::default(),
                                created_at: Set(Some(now)),
                                updated_at: Set(Some(now)),
                                draft_work_order_id: Set(saved.id),
                                component_id: Set(item.component_id),
                                variables: Set(vars_json),
                                final_cost: Set(final_cost),
                                extra_data: Set(extra_data),
                            };
                            if let Ok(line) = line_am.insert(&state.db).await {
                                let _ = tax_assoc::set_draft_material_line_taxes(
                                    &state.db,
                                    line.id,
                                    &tax_assoc::normalize_tax_ids(&item.tax_ids),
                                )
                                .await;
                            }
                        }
                    }
                }
            }

            sync_machine_lines(&state.db, saved.id, form.machine_lines, now).await;

            respond_create_modal_done::<WorkOrderCreateModalKey>(
                &htmx,
                &q.refresh_table(),
                &WorkOrderDetailRouteTag::new(saved.id).url(),
            )
        }
        Err(e) => {
            let customer_name =
                lariv_rs::plugins::customer::entities::customer::Entity::find_by_id(
                    form.customer_id,
                )
                .one(&state.db)
                .await
                .ok()
                .flatten()
                .map(|c| c.name)
                .unwrap_or_default();
            let components = fetch_components_meta(&state.db).await;
            let components_json =
                serde_json::to_string(&components).unwrap_or_else(|_| "[]".into());
            let machines_json = fetch_machines_json(&state.db).await;
            let page = WorkOrderCreateModalPage {
                form_name: q.form_name(),
                order_number: form.order_number,
                customer_id: Some(form.customer_id),
                customer_name,
                duration: form.duration.clone(),
                items_json: form.items.unwrap_or_default(),
                components_json,
                machine_lines_json: form.machine_lines.unwrap_or_default(),
                machines_json,
                taxes_json: tax_assoc::taxes_catalog_json(&state.db).await,
                error: e.to_string(),
            };
            html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response()
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct WorkOrderEditForm {
    #[serde(alias = "order_number", default)]
    pub order_number: String,
    #[serde(
        alias = "CustomerID",
        alias = "customer_id",
        default,
        deserialize_with = "i64_from_str_or_zero"
    )]
    pub customer_id: i64,
    #[serde(alias = "items", default)]
    pub items: Option<String>,
    #[serde(alias = "machine_lines", default)]
    pub machine_lines: Option<String>,
    #[serde(alias = "duration", default)]
    pub duration: String,
}

pub async fn work_order_edit_get(
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    Query(q): Query<ModalFormQuery>,
    Path(id): Path<i64>,
) -> Response {
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    let order = match draft_work_order::Entity::find_by_id(id)
        .one(&state.db)
        .await
    {
        Ok(Some(o)) => o,
        _ => return Redirect::to(&DraftWorkOrdersDefaultRouteTag.url()).into_response(),
    };
    let customer_name =
        lariv_rs::plugins::customer::entities::customer::Entity::find_by_id(order.customer_id)
            .one(&state.db)
            .await
            .ok()
            .flatten()
            .map(|c| c.name)
            .unwrap_or_default();

    let items_json = draft_material_lines_json(&state.db, id).await;
    let components = fetch_components_meta(&state.db).await;
    let components_json = serde_json::to_string(&components).unwrap_or_else(|_| "[]".into());
    let machines_json = fetch_machines_json(&state.db).await;
    let machine_lines_json = machine_lines_json(&state.db, id).await;

    let page = WorkOrderEditModalPage {
        id,
        form_name: q.form_name(),
        order_number: order.order_number,
        customer_id: order.customer_id,
        customer_name,
        duration: format_job_duration(order.duration),
        items_json,
        components_json,
        machine_lines_json,
        machines_json,
        taxes_json: tax_assoc::taxes_catalog_json(&state.db).await,
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response()
}

pub async fn work_order_edit_post(
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    htmx: Htmx,
    Query(q): Query<ModalFormQuery>,
    Path(id): Path<i64>,
    Form(form): Form<WorkOrderEditForm>,
) -> impl IntoResponse {
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    let existing = match draft_work_order::Entity::find_by_id(id)
        .one(&state.db)
        .await
    {
        Ok(Some(o)) => o,
        _ => return Redirect::to(&DraftWorkOrdersDefaultRouteTag.url()).into_response(),
    };

    if form.customer_id <= 0 {
        let components = fetch_components_meta(&state.db).await;
        let components_json = serde_json::to_string(&components).unwrap_or_else(|_| "[]".into());
        let machines_json = fetch_machines_json(&state.db).await;
        let page = WorkOrderEditModalPage {
            id,
            form_name: q.form_name(),
            order_number: form.order_number,
            customer_id: 0,
            customer_name: String::new(),
            duration: form.duration.clone(),
            items_json: form.items.unwrap_or_default(),
            components_json,
            machine_lines_json: form.machine_lines.unwrap_or_default(),
            machines_json,
            taxes_json: tax_assoc::taxes_catalog_json(&state.db).await,
            error: "Please select a customer.".into(),
        };
        return html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response();
    }

    if let Err(e) = validate_machine_lines_json(form.machine_lines.as_deref()) {
        let components = fetch_components_meta(&state.db).await;
        let components_json = serde_json::to_string(&components).unwrap_or_else(|_| "[]".into());
        let machines_json = fetch_machines_json(&state.db).await;
        let customer_name =
            lariv_rs::plugins::customer::entities::customer::Entity::find_by_id(form.customer_id)
                .one(&state.db)
                .await
                .ok()
                .flatten()
                .map(|c| c.name)
                .unwrap_or_default();
        let page = WorkOrderEditModalPage {
            id,
            form_name: q.form_name(),
            order_number: form.order_number,
            customer_id: form.customer_id,
            customer_name,
            duration: form.duration.clone(),
            items_json: form.items.unwrap_or_default(),
            components_json,
            machine_lines_json: form.machine_lines.unwrap_or_default(),
            machines_json,
            taxes_json: tax_assoc::taxes_catalog_json(&state.db).await,
            error: e,
        };
        return html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response();
    }

    let now = Utc::now();
    let duration = match parse_optional_job_duration(&form.duration, existing.duration) {
        Ok(d) => d,
        Err(e) => {
            let components = fetch_components_meta(&state.db).await;
            let components_json =
                serde_json::to_string(&components).unwrap_or_else(|_| "[]".into());
            let machines_json = fetch_machines_json(&state.db).await;
            let customer_name =
                lariv_rs::plugins::customer::entities::customer::Entity::find_by_id(
                    form.customer_id,
                )
                .one(&state.db)
                .await
                .ok()
                .flatten()
                .map(|c| c.name)
                .unwrap_or_default();
            let page = WorkOrderEditModalPage {
                id,
                form_name: q.form_name(),
                order_number: form.order_number,
                customer_id: form.customer_id,
                customer_name,
                duration: form.duration.clone(),
                items_json: form.items.unwrap_or_default(),
                components_json,
                machine_lines_json: form.machine_lines.unwrap_or_default(),
                machines_json,
                taxes_json: tax_assoc::taxes_catalog_json(&state.db).await,
                error: format!("Invalid duration: {e}"),
            };
            return html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response();
        }
    };
    let mut am: draft_work_order::ActiveModel = existing.into();
    am.updated_at = Set(Some(now));
    am.order_number = Set(form.order_number.trim().to_string());
    am.customer_id = Set(form.customer_id);
    am.duration = Set(duration);

    match am.update(&state.db).await {
        Ok(_) => {
            // Synchronize items if submitted
            if let Some(items_str) = form.items.as_deref().filter(|s| !s.trim().is_empty()) {
                if let Ok(items) =
                    serde_json::from_str::<Vec<super::forms::DraftWorkOrderLineInput>>(items_str)
                {
                    let _ = draft_work_order_material_line::Entity::delete_many()
                        .filter(draft_work_order_material_line::Column::DraftWorkOrderId.eq(id))
                        .exec(&state.db)
                        .await;

                    for item in items {
                        if item.component_id <= 0 {
                            continue;
                        }
                        if let Ok((vars_json, final_cost, extra_data)) =
                            resolve_and_compute_line_data(
                                &state.db,
                                item.component_id,
                                item.variables.as_ref(),
                                item.extra_data.as_ref(),
                            )
                            .await
                        {
                            let line_am = draft_work_order_material_line::ActiveModel {
                                id: Default::default(),
                                created_at: Set(Some(now)),
                                updated_at: Set(Some(now)),
                                draft_work_order_id: Set(id),
                                component_id: Set(item.component_id),
                                variables: Set(vars_json),
                                final_cost: Set(final_cost),
                                extra_data: Set(extra_data),
                            };
                            if let Ok(line) = line_am.insert(&state.db).await {
                                let _ = tax_assoc::set_draft_material_line_taxes(
                                    &state.db,
                                    line.id,
                                    &tax_assoc::normalize_tax_ids(&item.tax_ids),
                                )
                                .await;
                            }
                        }
                    }
                }
            }

            sync_machine_lines(&state.db, id, form.machine_lines, now).await;

            respond_edit_modal_done::<WorkOrderEditModalKey>(
                &htmx,
                &WorkOrderDetailRouteTag::new(id).url(),
            )
        }
        Err(e) => {
            let customer_name =
                lariv_rs::plugins::customer::entities::customer::Entity::find_by_id(
                    form.customer_id,
                )
                .one(&state.db)
                .await
                .ok()
                .flatten()
                .map(|c| c.name)
                .unwrap_or_default();
            let components = fetch_components_meta(&state.db).await;
            let components_json =
                serde_json::to_string(&components).unwrap_or_else(|_| "[]".into());
            let machines_json = fetch_machines_json(&state.db).await;
            let page = WorkOrderEditModalPage {
                id,
                form_name: q.form_name(),
                order_number: form.order_number,
                customer_id: form.customer_id,
                customer_name,
                duration: form.duration.clone(),
                items_json: form.items.unwrap_or_default(),
                components_json,
                machine_lines_json: form.machine_lines.unwrap_or_default(),
                machines_json,
                taxes_json: tax_assoc::taxes_catalog_json(&state.db).await,
                error: e.to_string(),
            };
            html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response()
        }
    }
}

pub async fn work_order_delete_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    Path(id): Path<i64>,
) -> maud::Markup {
    let page = ConfirmDeleteModalPage {
        modal_uid: WorkOrderDeleteModalKey::ID.to_string(),
        title: "Delete Work Order".into(),
        message: "Are you sure you want to delete this work order? All associated line items will also be deleted.".into(),
        post_url: WorkOrderDeletePostRouteTag::new(id).url(),
        error: String::new(),
    };
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    html_built_page_with_slots(&page, &chrome, &slot_ctx)
}

pub async fn work_order_delete_post(
    Cap(state): Cap<WorkOrdersState>,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let _ = draft_work_order::Entity::delete_by_id(id)
        .exec(&state.db)
        .await;
    htmx.redirect(&DraftWorkOrdersDefaultRouteTag.url())
}

// ==========================================
// 1b. DRAFT WORK ORDER LINES
// ==========================================

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct WorkOrderLineEditForm {
    #[serde(
        alias = "DraftWorkOrderID",
        alias = "draft_work_order_id",
        default,
        deserialize_with = "opt_i64_from_str"
    )]
    pub draft_work_order_id: Option<i64>,
    #[serde(
        alias = "ComponentID",
        alias = "component_id",
        default,
        deserialize_with = "i64_from_str_or_zero"
    )]
    pub component_id: i64,
    #[serde(alias = "variables", default)]
    pub variables: String,
    #[serde(alias = "quantity", default)]
    pub quantity: String,
    #[serde(alias = "extra_data", default)]
    pub extra_data: Option<String>,
    #[serde(alias = "taxes", default)]
    pub taxes: Vec<i64>,
}

pub async fn work_order_line_edit_get(
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    Query(q): Query<ModalFormQuery>,
    Path(id): Path<i64>,
) -> Response {
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    let line = match draft_work_order_material_line::Entity::find_by_id(id)
        .one(&state.db)
        .await
    {
        Ok(Some(l)) => l,
        _ => return Redirect::to(&DraftWorkOrdersDefaultRouteTag.url()).into_response(),
    };
    let order = draft_work_order::Entity::find_by_id(line.draft_work_order_id)
        .one(&state.db)
        .await
        .ok()
        .flatten();
    let label = order
        .map(|o| format!("{} (#{})", o.order_number, o.id))
        .unwrap_or_else(|| format!("#{}", line.draft_work_order_id));

    let comp = component::Entity::find_by_id(line.component_id)
        .one(&state.db)
        .await
        .ok()
        .flatten();
    let comp_label = comp.map(|c| c.name).unwrap_or_default();

    let extra_data = line.extra_data_str();
    let tax_ids = tax_assoc::load_draft_material_line_tax_ids(&state.db, line.id)
        .await
        .unwrap_or_default();
    let tax_items = tax_items_for_ids(&state.db, &tax_ids).await;
    let page = WorkOrderLineEditModalPage {
        id,
        draft_work_order_id: line.draft_work_order_id,
        draft_work_order_label: label,
        form_name: q.form_name(),
        component_id: line.component_id,
        component_label: comp_label,
        variables: serde_json::to_string(&line.variables).unwrap_or_else(|_| "{}".into()),
        extra_data,
        tax_items,
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response()
}

pub async fn work_order_line_edit_post(
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    htmx: Htmx,
    Query(q): Query<ModalFormQuery>,
    Path(id): Path<i64>,
    Form(form): Form<WorkOrderLineEditForm>,
) -> impl IntoResponse {
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    let existing = match draft_work_order_material_line::Entity::find_by_id(id)
        .one(&state.db)
        .await
    {
        Ok(Some(l)) => l,
        _ => return Redirect::to(&DraftWorkOrdersDefaultRouteTag.url()).into_response(),
    };
    let target_order_id = form
        .draft_work_order_id
        .unwrap_or(existing.draft_work_order_id);

    if form.component_id <= 0 {
        let order = draft_work_order::Entity::find_by_id(target_order_id)
            .one(&state.db)
            .await
            .ok()
            .flatten();
        let label = order
            .map(|o| format!("{} (#{})", o.order_number, o.id))
            .unwrap_or_else(|| format!("#{}", target_order_id));
        let page = WorkOrderLineEditModalPage {
            id,
            draft_work_order_id: target_order_id,
            draft_work_order_label: label,
            form_name: q.form_name(),
            component_id: form.component_id,
            component_label: String::new(),
            variables: form.variables,
            extra_data: form.extra_data.unwrap_or_default(),
            tax_items: tax_items_for_ids(&state.db, &form.taxes).await,
            error: "Please select a component.".into(),
        };
        return html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response();
    }

    let comp_label = component::Entity::find_by_id(form.component_id)
        .one(&state.db)
        .await
        .ok()
        .flatten()
        .map(|c| c.name)
        .unwrap_or_default();

    let vars_val = serde_json::Value::String(form.variables.clone());
    let extra_val = form.extra_data.clone().map(serde_json::Value::String);

    match resolve_and_compute_line_data(
        &state.db,
        form.component_id,
        Some(&vars_val),
        extra_val.as_ref(),
    )
    .await
    {
        Ok((vars_json, final_cost, extra_data)) => {
            let now = Utc::now();
            let mut am: draft_work_order_material_line::ActiveModel = existing.clone().into();
            am.updated_at = Set(Some(now));
            am.draft_work_order_id = Set(target_order_id);
            am.component_id = Set(form.component_id);
            am.variables = Set(vars_json);
            am.final_cost = Set(final_cost);
            am.extra_data = Set(extra_data);

            match am.update(&state.db).await {
                Ok(_) => {
                    let _ = tax_assoc::set_draft_material_line_taxes(
                        &state.db,
                        id,
                        &tax_assoc::normalize_tax_ids(&form.taxes),
                    )
                    .await;
                    respond_edit_modal_done::<WorkOrderLineEditModalKey>(
                        &htmx,
                        &WorkOrderDetailRouteTag::new(target_order_id).url(),
                    )
                }
                Err(e) => {
                    let order = draft_work_order::Entity::find_by_id(target_order_id)
                        .one(&state.db)
                        .await
                        .ok()
                        .flatten();
                    let label = order
                        .map(|o| format!("{} (#{})", o.order_number, o.id))
                        .unwrap_or_else(|| format!("#{}", target_order_id));
                    let page = WorkOrderLineEditModalPage {
                        id,
                        draft_work_order_id: target_order_id,
                        draft_work_order_label: label,
                        form_name: q.form_name(),
                        component_id: form.component_id,
                        component_label: comp_label,
                        variables: form.variables,
                        extra_data: form.extra_data.unwrap_or_default(),
                        tax_items: tax_items_for_ids(&state.db, &form.taxes).await,
                        error: e.to_string(),
                    };
                    html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response()
                }
            }
        }
        Err(err) => {
            let order = draft_work_order::Entity::find_by_id(target_order_id)
                .one(&state.db)
                .await
                .ok()
                .flatten();
            let label = order
                .map(|o| format!("{} (#{})", o.order_number, o.id))
                .unwrap_or_else(|| format!("#{}", target_order_id));
            let page = WorkOrderLineEditModalPage {
                id,
                draft_work_order_id: target_order_id,
                draft_work_order_label: label,
                form_name: q.form_name(),
                component_id: form.component_id,
                component_label: comp_label,
                variables: form.variables,
                extra_data: form.extra_data.unwrap_or_default(),
                tax_items: tax_items_for_ids(&state.db, &form.taxes).await,
                error: err,
            };
            html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response()
        }
    }
}

pub async fn work_order_line_delete_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    Path(id): Path<i64>,
) -> maud::Markup {
    let page = ConfirmDeleteModalPage {
        modal_uid: WorkOrderLineDeleteModalKey::ID.to_string(),
        title: "Delete Line Item".into(),
        message: "Are you sure you want to delete this line item?".into(),
        post_url: WorkOrderLineDeletePostRouteTag::new(id).url(),
        error: String::new(),
    };
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    html_built_page_with_slots(&page, &chrome, &slot_ctx)
}

pub async fn work_order_line_delete_post(
    Cap(state): Cap<WorkOrdersState>,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let draft_work_order_id = if let Ok(Some(line)) =
        draft_work_order_material_line::Entity::find_by_id(id)
            .one(&state.db)
            .await
    {
        let wid = line.draft_work_order_id;
        let _ = draft_work_order_material_line::Entity::delete_by_id(id)
            .exec(&state.db)
            .await;
        wid
    } else {
        return htmx.redirect(&DraftWorkOrdersDefaultRouteTag.url());
    };
    htmx.redirect(&WorkOrderDetailRouteTag::new(draft_work_order_id).url())
}

// ==========================================
// 1c. DRAFT WORK ORDER MACHINE LINES
// ==========================================

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct WorkOrderMachineLineFormData {
    #[serde(
        alias = "DraftWorkOrderID",
        alias = "draft_work_order_id",
        default,
        deserialize_with = "opt_i64_from_str"
    )]
    pub draft_work_order_id: Option<i64>,
    #[serde(
        alias = "MachineID",
        alias = "machine_id",
        default,
        deserialize_with = "i64_from_str_or_zero"
    )]
    pub machine_id: i64,
    #[serde(alias = "Variables", alias = "variables", default)]
    pub variables: String,
    #[serde(alias = "Rate", alias = "rate", default)]
    pub rate: String,
    #[serde(alias = "Duration", alias = "duration", default)]
    pub duration: String,
    #[serde(alias = "Taxes", alias = "taxes", default)]
    pub taxes: Vec<i64>,
}

async fn machine_line_machine_label(db: &sea_orm::DatabaseConnection, machine_id: i64) -> String {
    machine::Entity::find_by_id(machine_id)
        .one(db)
        .await
        .ok()
        .flatten()
        .map(|m| m.name)
        .unwrap_or_default()
}

async fn fetch_machines_json(db: &sea_orm::DatabaseConnection) -> String {
    let machines = machine::Entity::find()
        .order_by_asc(machine::Column::Name)
        .all(db)
        .await
        .unwrap_or_default();
    let rows: Vec<serde_json::Value> = machines
        .into_iter()
        .map(|m| {
            serde_json::json!({
                "id": m.id,
                "name": m.name,
                "variables": m.variables,
                "cost_formula": m.cost_formula,
            })
        })
        .collect();
    serde_json::to_string(&rows).unwrap_or_else(|_| "[]".into())
}

fn taxed_draft_grand_total(
    lines: &[draft_work_order_material_line::Model],
    machine_lines: &[draft_work_order_machine_line::Model],
    material_taxes: &HashMap<i64, Vec<lariv_rs::plugins::finance_taxes::entities::tax::Model>>,
    machine_taxes: &HashMap<i64, Vec<lariv_rs::plugins::finance_taxes::entities::tax::Model>>,
) -> Decimal {
    let materials: Decimal = lines
        .iter()
        .map(|l| l.taxed_total(tax_assoc::taxes_for_line(material_taxes, l.id)))
        .sum();
    let machines: Decimal = machine_lines
        .iter()
        .map(|l| l.taxed_total(tax_assoc::taxes_for_line(machine_taxes, l.id)))
        .sum();
    materials + machines
}

fn taxed_quotation_grand_total(
    material_lines: &[quotation_material_line::Model],
    machine_lines: &[quotation_machine_line::Model],
    material_taxes: &HashMap<i64, Vec<lariv_rs::plugins::finance_taxes::entities::tax::Model>>,
    machine_taxes: &HashMap<i64, Vec<lariv_rs::plugins::finance_taxes::entities::tax::Model>>,
) -> Decimal {
    let materials: Decimal = material_lines
        .iter()
        .map(|l| l.taxed_total(tax_assoc::taxes_for_line(material_taxes, l.id)))
        .sum();
    let machines: Decimal = machine_lines
        .iter()
        .map(|l| l.taxed_total(tax_assoc::taxes_for_line(machine_taxes, l.id)))
        .sum();
    materials + machines
}

async fn tax_items_for_ids(db: &sea_orm::DatabaseConnection, ids: &[i64]) -> Vec<ManyToManyItem> {
    let taxes = lariv_rs::plugins::finance_taxes::scope::load_taxes_by_ids(db, ids)
        .await
        .unwrap_or_default();
    taxes
        .into_iter()
        .map(|t| {
            ManyToManyItem::new(
                t.id.to_string(),
                lariv_rs::plugins::finance_taxes::scope::tax_label(&t),
            )
        })
        .collect()
}

async fn draft_material_lines_json(db: &sea_orm::DatabaseConnection, order_id: i64) -> String {
    let lines = draft_work_order_material_line::Entity::find()
        .filter(draft_work_order_material_line::Column::DraftWorkOrderId.eq(order_id))
        .order_by_asc(draft_work_order_material_line::Column::Id)
        .all(db)
        .await
        .unwrap_or_default();
    let ids: Vec<i64> = lines.iter().map(|l| l.id).collect();
    let tax_map = tax_assoc::load_draft_material_line_tax_ids_map(db, &ids)
        .await
        .unwrap_or_default();
    let rows: Vec<serde_json::Value> = lines
        .into_iter()
        .map(|l| {
            let mut v = serde_json::to_value(&l).unwrap_or_else(|_| serde_json::json!({}));
            if let serde_json::Value::Object(ref mut m) = v {
                m.insert(
                    "tax_ids".into(),
                    serde_json::json!(tax_map.get(&l.id).cloned().unwrap_or_default()),
                );
            }
            v
        })
        .collect();
    serde_json::to_string(&rows).unwrap_or_else(|_| "[]".into())
}

async fn machine_lines_json(db: &sea_orm::DatabaseConnection, order_id: i64) -> String {
    let machine_lines = draft_work_order_machine_line::Entity::find()
        .filter(draft_work_order_machine_line::Column::DraftWorkOrderId.eq(order_id))
        .order_by_asc(draft_work_order_machine_line::Column::Id)
        .all(db)
        .await
        .unwrap_or_default();
    let ids: Vec<i64> = machine_lines.iter().map(|l| l.id).collect();
    let tax_map = tax_assoc::load_draft_machine_line_tax_ids_map(db, &ids)
        .await
        .unwrap_or_default();
    let rows: Vec<serde_json::Value> = machine_lines
        .into_iter()
        .map(|l| {
            serde_json::json!({
                "id": l.id,
                "db_id": l.id,
                "machine_id": l.machine_id,
                "variables": l.variables,
                "tax_ids": tax_map.get(&l.id).cloned().unwrap_or_default(),
            })
        })
        .collect();
    serde_json::to_string(&rows).unwrap_or_else(|_| "[]".into())
}

fn validate_machine_lines_json(ml_str: Option<&str>) -> Result<(), String> {
    let Some(s) = ml_str.filter(|s| !s.trim().is_empty()) else {
        return Ok(());
    };
    let _: Vec<super::forms::DraftWorkOrderMachineLineInput> =
        serde_json::from_str(s).map_err(|_| "Invalid machine lines data.".to_string())?;
    Ok(())
}

async fn latest_child_id(
    db: &sea_orm::DatabaseConnection,
    table: &str,
    parent_col: &str,
    parent_id: i64,
) -> Option<i64> {
    let backend = db.get_database_backend();
    let ph = match backend {
        sea_orm::DatabaseBackend::Postgres => "$1",
        _ => "?",
    };
    let sql = format!("SELECT id FROM {table} WHERE {parent_col} = {ph} ORDER BY id DESC LIMIT 1");
    let row = db
        .query_one_raw(Statement::from_sql_and_values(
            backend,
            sql,
            [parent_id.into()],
        ))
        .await
        .ok()
        .flatten()?;
    row.try_get::<i64>("", "id").ok()
}

async fn sync_machine_lines(
    db: &sea_orm::DatabaseConnection,
    order_id: i64,
    ml_str: Option<String>,
    now: chrono::DateTime<chrono::Utc>,
) {
    let Some(s) = ml_str.as_deref().filter(|s| !s.trim().is_empty()) else {
        return;
    };
    let Ok(lines) = serde_json::from_str::<Vec<super::forms::DraftWorkOrderMachineLineInput>>(s)
    else {
        return;
    };
    let _ = draft_work_order_machine_line::Entity::delete_many()
        .filter(draft_work_order_machine_line::Column::DraftWorkOrderId.eq(order_id))
        .exec(db)
        .await;
    for line in lines {
        if line.machine_id <= 0 {
            continue;
        }
        let Ok((vars_json, final_cost, _)) =
            resolve_and_compute_machine_cost(db, line.machine_id, line.variables.as_ref()).await
        else {
            continue;
        };
        let am = draft_work_order_machine_line::ActiveModel {
            id: Default::default(),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            draft_work_order_id: Set(order_id),
            machine_id: Set(line.machine_id),
            variables: Set(vars_json),
            final_cost: Set(final_cost),
        };
        let line_id = match am.insert(db).await {
            Ok(saved) => Some(saved.id),
            Err(_) => {
                latest_child_id(
                    db,
                    "draft_work_order_machine_lines",
                    "draft_work_order_id",
                    order_id,
                )
                .await
            }
        };
        if let Some(line_id) = line_id {
            let _ = tax_assoc::set_draft_machine_line_taxes(
                db,
                line_id,
                &tax_assoc::normalize_tax_ids(&line.tax_ids),
            )
            .await;
        }
    }
}

async fn components_json(db: &sea_orm::DatabaseConnection) -> String {
    let components = fetch_components_meta(db).await;
    serde_json::to_string(&components).unwrap_or_else(|_| "[]".into())
}

async fn invoice_material_lines_json(db: &sea_orm::DatabaseConnection, invoice_id: i64) -> String {
    let lines = quotation_material_line::Entity::find()
        .filter(quotation_material_line::Column::InvoiceId.eq(invoice_id))
        .order_by_asc(quotation_material_line::Column::Id)
        .all(db)
        .await
        .unwrap_or_default();
    let ids: Vec<i64> = lines.iter().map(|l| l.id).collect();
    let tax_map = tax_assoc::load_quotation_material_line_tax_ids_map(db, &ids)
        .await
        .unwrap_or_default();
    let rows: Vec<serde_json::Value> = lines
        .into_iter()
        .map(|l| {
            let mut v = serde_json::to_value(&l).unwrap_or_else(|_| serde_json::json!({}));
            if let serde_json::Value::Object(ref mut m) = v {
                m.insert(
                    "tax_ids".into(),
                    serde_json::json!(tax_map.get(&l.id).cloned().unwrap_or_default()),
                );
            }
            v
        })
        .collect();
    serde_json::to_string(&rows).unwrap_or_else(|_| "[]".into())
}

async fn invoice_machine_lines_json(db: &sea_orm::DatabaseConnection, invoice_id: i64) -> String {
    let lines = quotation_machine_line::Entity::find()
        .filter(quotation_machine_line::Column::InvoiceId.eq(invoice_id))
        .order_by_asc(quotation_machine_line::Column::Id)
        .all(db)
        .await
        .unwrap_or_default();
    let ids: Vec<i64> = lines.iter().map(|l| l.id).collect();
    let tax_map = tax_assoc::load_quotation_machine_line_tax_ids_map(db, &ids)
        .await
        .unwrap_or_default();
    let rows: Vec<serde_json::Value> = lines
        .into_iter()
        .map(|l| {
            serde_json::json!({
                "id": l.id,
                "db_id": l.id,
                "machine_id": l.machine_id,
                "variables": l.variables,
                "tax_ids": tax_map.get(&l.id).cloned().unwrap_or_default(),
            })
        })
        .collect();
    serde_json::to_string(&rows).unwrap_or_else(|_| "[]".into())
}

async fn sync_invoice_material_lines(
    db: &sea_orm::DatabaseConnection,
    invoice_id: i64,
    lines_str: Option<String>,
    now: chrono::DateTime<chrono::Utc>,
) {
    let Some(s) = lines_str.as_deref().filter(|s| !s.trim().is_empty()) else {
        return;
    };
    let Ok(items) = serde_json::from_str::<Vec<super::forms::DraftWorkOrderLineInput>>(s) else {
        return;
    };
    let _ = quotation_material_line::Entity::delete_many()
        .filter(quotation_material_line::Column::InvoiceId.eq(invoice_id))
        .exec(db)
        .await;
    for item in items {
        if item.component_id <= 0 {
            continue;
        }
        if let Ok((vars_json, final_cost, extra_data)) = resolve_and_compute_line_data(
            db,
            item.component_id,
            item.variables.as_ref(),
            item.extra_data.as_ref(),
        )
        .await
        {
            let line_am = quotation_material_line::ActiveModel {
                id: Default::default(),
                created_at: Set(Some(now)),
                updated_at: Set(Some(now)),
                invoice_id: Set(invoice_id),
                component_id: Set(item.component_id),
                variables: Set(vars_json),
                final_cost: Set(final_cost),
                extra_data: Set(extra_data),
            };
            if let Ok(saved) = line_am.insert(db).await {
                let _ = tax_assoc::set_quotation_material_line_taxes(
                    db,
                    saved.id,
                    &tax_assoc::normalize_tax_ids(&item.tax_ids),
                )
                .await;
            }
        }
    }
}

async fn sync_invoice_machine_lines(
    db: &sea_orm::DatabaseConnection,
    invoice_id: i64,
    ml_str: Option<String>,
    now: chrono::DateTime<chrono::Utc>,
) {
    let Some(s) = ml_str.as_deref().filter(|s| !s.trim().is_empty()) else {
        return;
    };
    let Ok(lines) = serde_json::from_str::<Vec<super::forms::DraftWorkOrderMachineLineInput>>(s)
    else {
        return;
    };
    let _ = quotation_machine_line::Entity::delete_many()
        .filter(quotation_machine_line::Column::InvoiceId.eq(invoice_id))
        .exec(db)
        .await;
    for line in lines {
        if line.machine_id <= 0 {
            continue;
        }
        let Ok((vars_json, final_cost, machine_name)) =
            resolve_and_compute_machine_cost(db, line.machine_id, line.variables.as_ref()).await
        else {
            continue;
        };
        let am = quotation_machine_line::ActiveModel {
            id: Default::default(),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            invoice_id: Set(invoice_id),
            machine_id: Set(Some(line.machine_id)),
            name: Set(machine_name),
            variables: Set(vars_json),
            final_cost: Set(final_cost),
        };
        let line_id = match am.insert(db).await {
            Ok(saved) => Some(saved.id),
            Err(_) => {
                latest_child_id(db, "kds_quotation_machine_lines", "invoice_id", invoice_id).await
            }
        };
        if let Some(line_id) = line_id {
            let _ = tax_assoc::set_quotation_machine_line_taxes(
                db,
                line_id,
                &tax_assoc::normalize_tax_ids(&line.tax_ids),
            )
            .await;
        }
    }
}

async fn sync_invoice_lines(
    db: &sea_orm::DatabaseConnection,
    invoice_id: i64,
    material_lines_str: Option<String>,
    machine_lines_str: Option<String>,
    now: chrono::DateTime<chrono::Utc>,
) {
    sync_invoice_material_lines(db, invoice_id, material_lines_str, now).await;
    sync_invoice_machine_lines(db, invoice_id, machine_lines_str, now).await;
}

pub async fn work_order_machine_line_edit_get(
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    Query(q): Query<ModalFormQuery>,
    Path(id): Path<i64>,
) -> Response {
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    let line = match draft_work_order_machine_line::Entity::find_by_id(id)
        .one(&state.db)
        .await
    {
        Ok(Some(l)) => l,
        _ => return Redirect::to(&DraftWorkOrdersDefaultRouteTag.url()).into_response(),
    };
    let order = draft_work_order::Entity::find_by_id(line.draft_work_order_id)
        .one(&state.db)
        .await
        .ok()
        .flatten();
    let label = order
        .map(|o| format!("{} (#{})", o.order_number, o.id))
        .unwrap_or_else(|| format!("#{}", line.draft_work_order_id));

    let machine_label = machine_line_machine_label(&state.db, line.machine_id).await;
    let tax_ids = tax_assoc::load_draft_machine_line_tax_ids(&state.db, line.id)
        .await
        .unwrap_or_default();
    let tax_items = tax_items_for_ids(&state.db, &tax_ids).await;

    let vars_str = serde_json::to_string(&line.variables).unwrap_or_else(|_| "{}".into());
    let page = WorkOrderMachineLineEditModalPage {
        id,
        form_name: q.form_name(),
        draft_work_order_id: line.draft_work_order_id,
        draft_work_order_label: label,
        machine_id: line.machine_id,
        machine_label,
        variables: vars_str,
        tax_items,
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response()
}

pub async fn work_order_machine_line_edit_post(
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    htmx: Htmx,
    Query(q): Query<ModalFormQuery>,
    Path(id): Path<i64>,
    Form(form): Form<WorkOrderMachineLineFormData>,
) -> Response {
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    let existing = match draft_work_order_machine_line::Entity::find_by_id(id)
        .one(&state.db)
        .await
    {
        Ok(Some(l)) => l,
        _ => return Redirect::to(&DraftWorkOrdersDefaultRouteTag.url()).into_response(),
    };
    let target_order_id = form
        .draft_work_order_id
        .unwrap_or(existing.draft_work_order_id);
    let vars_raw = if form.variables.trim().is_empty() {
        form.duration.clone()
    } else {
        form.variables.clone()
    };

    if form.machine_id <= 0 {
        return late_render_edit_error(
            &state.db,
            &chrome,
            &slot_ctx,
            &q,
            id,
            target_order_id,
            "Please select a machine.".into(),
            0,
            String::new(),
            vars_raw,
            form.taxes.clone(),
        )
        .await;
    }

    let machine_label = machine_line_machine_label(&state.db, form.machine_id).await;
    let vars_val = serde_json::Value::String(vars_raw.clone());
    let computed =
        resolve_and_compute_machine_cost(&state.db, form.machine_id, Some(&vars_val)).await;

    let (vars_json, final_cost) = match computed {
        Ok((v, c, _)) => (v, c),
        Err(e) => {
            return late_render_edit_error(
                &state.db,
                &chrome,
                &slot_ctx,
                &q,
                id,
                target_order_id,
                e,
                form.machine_id,
                machine_label,
                vars_raw,
                form.taxes.clone(),
            )
            .await;
        }
    };

    let now = Utc::now();
    let mut am: draft_work_order_machine_line::ActiveModel = existing.into();
    am.updated_at = Set(Some(now));
    am.draft_work_order_id = Set(target_order_id);
    am.machine_id = Set(form.machine_id);
    am.variables = Set(vars_json);
    am.final_cost = Set(final_cost);

    match am.update(&state.db).await {
        Ok(_) => {
            let _ = tax_assoc::set_draft_machine_line_taxes(
                &state.db,
                id,
                &tax_assoc::normalize_tax_ids(&form.taxes),
            )
            .await;
            respond_edit_modal_done::<WorkOrderMachineLineEditModalKey>(
                &htmx,
                &WorkOrderDetailRouteTag::new(target_order_id).url(),
            )
        }
        Err(e) => {
            late_render_edit_error(
                &state.db,
                &chrome,
                &slot_ctx,
                &q,
                id,
                target_order_id,
                e.to_string(),
                form.machine_id,
                machine_label,
                vars_raw,
                form.taxes.clone(),
            )
            .await
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn late_render_edit_error(
    db: &sea_orm::DatabaseConnection,
    chrome: &SharedChromeFolder,
    slot_ctx: &SlotCtx,
    q: &ModalFormQuery,
    id: i64,
    target_order_id: i64,
    error: String,
    machine_id: i64,
    machine_label: String,
    variables: String,
    taxes: Vec<i64>,
) -> Response {
    let order = draft_work_order::Entity::find_by_id(target_order_id)
        .one(db)
        .await
        .ok()
        .flatten();
    let label = order
        .map(|o| format!("{} (#{})", o.order_number, o.id))
        .unwrap_or_else(|| format!("#{}", target_order_id));
    let tax_items = tax_items_for_ids(db, &taxes).await;
    let page = WorkOrderMachineLineEditModalPage {
        id,
        form_name: q.form_name(),
        draft_work_order_id: target_order_id,
        draft_work_order_label: label,
        machine_id,
        machine_label,
        variables,
        tax_items,
        error,
    };
    html_built_page_with_slots(&page, chrome, slot_ctx).into_response()
}

pub async fn work_order_machine_line_delete_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    Path(id): Path<i64>,
) -> maud::Markup {
    let page = ConfirmDeleteModalPage {
        modal_uid: WorkOrderMachineLineDeleteModalKey::ID.to_string(),
        title: "Delete Machine Line".into(),
        message: "Are you sure you want to delete this machine line?".into(),
        post_url: WorkOrderMachineLineDeletePostRouteTag::new(id).url(),
        error: String::new(),
    };
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    html_built_page_with_slots(&page, &chrome, &slot_ctx)
}

pub async fn work_order_machine_line_delete_post(
    Cap(state): Cap<WorkOrdersState>,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let draft_work_order_id = if let Ok(Some(line)) =
        draft_work_order_machine_line::Entity::find_by_id(id)
            .one(&state.db)
            .await
    {
        let wid = line.draft_work_order_id;
        let _ = draft_work_order_machine_line::Entity::delete_by_id(id)
            .exec(&state.db)
            .await;
        wid
    } else {
        return htmx.redirect(&DraftWorkOrdersDefaultRouteTag.url());
    };
    htmx.redirect(&WorkOrderDetailRouteTag::new(draft_work_order_id).url())
}

pub async fn component_select(
    Cap(state): Cap<WorkOrdersState>,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<EntitySelectQuery>,
) -> maud::Markup {
    let mut query = component::Entity::find();
    if let Some(n) = q.name.as_deref().filter(|s| !s.trim().is_empty()) {
        query = query.filter(component::Column::Name.contains(n));
    }
    let components = query
        .order_by_desc(component::Column::Id)
        .all(&state.db)
        .await
        .unwrap_or_default();

    let page = ComponentSelectPage {
        components,
        target_input: q.target_input.unwrap_or_else(|| "component_id".into()),
        path_and_query: uri.to_string(),
    };
    respond_picker_select::<ComponentSelectTableKey, ComponentSelectModalKey, _>(&htmx, &page)
}

// ==========================================
// 2. COMPONENTS
// ==========================================

pub async fn components_list(
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    htmx: Htmx,
    uri: Uri,
) -> maud::Markup {
    let _ = crate::work_orders::seed::ensure_standard_seeds(&state.db).await;
    let comps = component::Entity::find()
        .order_by_asc(component::Column::Name)
        .all(&state.db)
        .await
        .unwrap_or_default();

    let page = ComponentListPage {
        components: comps,
        path_and_query: path_and_query(&uri),
    };
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    if htmx.targets::<ComponentTableKey>() {
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

pub async fn component_detail(
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(comp) = component::Entity::find_by_id(id)
        .one(&state.db)
        .await
        .unwrap_or(None)
    else {
        return Redirect::to(&WorkOrdersComponentsRouteTag.url()).into_response();
    };

    let page = ComponentDetailPage { component: comp };
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx).into_response()
}

pub use super::forms::{ComponentCreateForm, ComponentEditForm, ComponentForm};

fn empty_component_create_page(
    q: &ModalFormQuery,
    name: String,
    cost_formula: String,
    weight_formula: String,
    variables: Vec<String>,
    error: String,
) -> ComponentCreateModalPage {
    ComponentCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        target_input: q.target_input(),
        name,
        variables,
        cost_formula,
        weight_formula,
        error,
    }
}

fn empty_component_edit_page(
    id: i64,
    q: &ModalFormQuery,
    name: String,
    cost_formula: String,
    weight_formula: String,
    variables: Vec<String>,
    error: String,
) -> ComponentEditModalPage {
    ComponentEditModalPage {
        id,
        form_name: q.form_name(),
        name,
        variables,
        cost_formula,
        weight_formula,
        error,
    }
}

pub async fn component_create_get(
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    Query(q): Query<ModalFormQuery>,
) -> maud::Markup {
    let _ = crate::work_orders::seed::ensure_standard_seeds(&state.db).await;
    let page = empty_component_create_page(
        &q,
        String::new(),
        String::new(),
        String::new(),
        Vec::new(),
        String::new(),
    );
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    html_built_page_with_slots(&page, &chrome, &slot_ctx)
}

pub async fn component_create_post(
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    htmx: Htmx,
    Query(q): Query<ModalFormQuery>,
    HtmlFormBody(form): HtmlFormBody<ComponentForm>,
) -> Response {
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();

    if form.name.trim().is_empty() {
        let page = empty_component_create_page(
            &q,
            form.name,
            form.cost_formula,
            form.weight_formula,
            form.variables,
            "Component name is required.".into(),
        );
        return html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response();
    }

    let variables = match parse_component_variables_list(&form.variables) {
        Ok(v) => v,
        Err(e) => {
            let page = empty_component_create_page(
                &q,
                form.name,
                form.cost_formula,
                form.weight_formula,
                form.variables,
                e,
            );
            return html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response();
        }
    };

    let probe = component_from_form_fields(
        form.name.trim().to_string(),
        form.cost_formula.clone(),
        form.weight_formula.clone(),
        variables.clone(),
    );
    if let Err(e) = probe.validate_formulas() {
        let page = empty_component_create_page(
            &q,
            form.name,
            form.cost_formula,
            form.weight_formula,
            form.variables,
            e.to_string(),
        );
        return html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response();
    }

    let now = Utc::now();
    let model = component::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        name: Set(form.name.trim().to_string()),
        cost_formula: Set(form.cost_formula),
        weight_formula: Set(form.weight_formula),
        variables: Set(variables),
    };

    match model.insert(&state.db).await {
        Ok(saved) => respond_create_modal_done_fk::<ComponentCreateModalKey>(
            &htmx,
            &q.refresh_table(),
            &ComponentDetailRouteTag::new(saved.id).url(),
            saved.id,
            &saved.name,
            &q.target_input(),
        ),
        Err(e) => {
            let page = empty_component_create_page(
                &q,
                form.name,
                probe.cost_formula,
                probe.weight_formula,
                form.variables,
                e.to_string(),
            );
            html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response()
        }
    }
}

pub async fn component_edit_get(
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    Query(q): Query<ModalFormQuery>,
    Path(id): Path<i64>,
) -> Response {
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    let comp = match component::Entity::find_by_id(id).one(&state.db).await {
        Ok(Some(c)) => c,
        _ => return Redirect::to(&WorkOrdersComponentsRouteTag.url()).into_response(),
    };

    let page = empty_component_edit_page(
        id,
        &q,
        comp.name,
        comp.cost_formula,
        comp.weight_formula,
        super::forms::schema_entries_from_json(&comp.variables),
        String::new(),
    );
    html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response()
}

pub async fn component_edit_post(
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    htmx: Htmx,
    Query(q): Query<ModalFormQuery>,
    Path(id): Path<i64>,
    HtmlFormBody(form): HtmlFormBody<ComponentForm>,
) -> impl IntoResponse {
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    let existing = match component::Entity::find_by_id(id).one(&state.db).await {
        Ok(Some(c)) => c,
        _ => return Redirect::to(&WorkOrdersComponentsRouteTag.url()).into_response(),
    };

    if form.name.trim().is_empty() {
        let page = empty_component_edit_page(
            id,
            &q,
            form.name,
            form.cost_formula,
            form.weight_formula,
            form.variables,
            "Component name is required.".into(),
        );
        return html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response();
    }

    let variables = match parse_component_variables_list(&form.variables) {
        Ok(v) => v,
        Err(e) => {
            let page = empty_component_edit_page(
                id,
                &q,
                form.name,
                form.cost_formula,
                form.weight_formula,
                form.variables,
                e,
            );
            return html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response();
        }
    };

    let probe = component_from_form_fields(
        form.name.trim().to_string(),
        form.cost_formula.clone(),
        form.weight_formula.clone(),
        variables.clone(),
    );
    if let Err(e) = probe.validate_formulas() {
        let page = empty_component_edit_page(
            id,
            &q,
            form.name,
            form.cost_formula,
            form.weight_formula,
            form.variables,
            e.to_string(),
        );
        return html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response();
    }

    let now = Utc::now();
    let mut am: component::ActiveModel = existing.into();
    am.updated_at = Set(Some(now));
    am.name = Set(form.name.trim().to_string());
    am.cost_formula = Set(form.cost_formula);
    am.weight_formula = Set(form.weight_formula);
    am.variables = Set(variables);

    match am.update(&state.db).await {
        Ok(_) => respond_edit_modal_done::<ComponentEditModalKey>(
            &htmx,
            &ComponentDetailRouteTag::new(id).url(),
        ),
        Err(e) => {
            let page = empty_component_edit_page(
                id,
                &q,
                form.name,
                probe.cost_formula,
                probe.weight_formula,
                form.variables,
                e.to_string(),
            );
            html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response()
        }
    }
}

pub async fn component_delete_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    Path(id): Path<i64>,
) -> maud::Markup {
    let page = ConfirmDeleteModalPage {
        modal_uid: ComponentDeleteModalKey::ID.to_string(),
        title: "Delete Component".into(),
        message: "Are you sure you want to delete this component?".into(),
        post_url: ComponentDeletePostRouteTag::new(id).url(),
        error: String::new(),
    };
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    html_built_page_with_slots(&page, &chrome, &slot_ctx)
}

pub async fn component_delete_post(
    Cap(state): Cap<WorkOrdersState>,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let _ = component::Entity::delete_by_id(id).exec(&state.db).await;
    htmx.redirect(&WorkOrdersComponentsRouteTag.url())
}

// ==========================================
// 7. QUOTATIONS
// ==========================================

pub async fn invoices_list(
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    htmx: Htmx,
    uri: Uri,
) -> maud::Markup {
    let invoices = quotation::Entity::find()
        .order_by_desc(quotation::Column::Date)
        .order_by_desc(quotation::Column::Id)
        .all(&state.db)
        .await
        .unwrap_or_default();

    let mut customer_names = Vec::with_capacity(invoices.len());
    let mut grand_totals = Vec::with_capacity(invoices.len());
    for inv in &invoices {
        let name =
            lariv_rs::plugins::customer::entities::customer::Entity::find_by_id(inv.customer_id)
                .one(&state.db)
                .await
                .ok()
                .flatten()
                .map(|c| c.name)
                .unwrap_or_else(|| format!("Customer #{}", inv.customer_id));
        customer_names.push(name);
        let mlines = quotation_material_line::Entity::find()
            .filter(quotation_material_line::Column::InvoiceId.eq(inv.id))
            .all(&state.db)
            .await
            .unwrap_or_default();
        let mlmach = quotation_machine_line::Entity::find()
            .filter(quotation_machine_line::Column::InvoiceId.eq(inv.id))
            .all(&state.db)
            .await
            .unwrap_or_default();
        let mat_ids: Vec<i64> = mlines.iter().map(|l| l.id).collect();
        let mach_ids: Vec<i64> = mlmach.iter().map(|l| l.id).collect();
        let mat_tax_ids = tax_assoc::load_quotation_material_line_tax_ids_map(&state.db, &mat_ids)
            .await
            .unwrap_or_default();
        let mach_tax_ids = tax_assoc::load_quotation_machine_line_tax_ids_map(&state.db, &mach_ids)
            .await
            .unwrap_or_default();
        let mat_taxes = tax_assoc::resolve_taxes_by_line_id(&state.db, &mat_tax_ids).await;
        let mach_taxes = tax_assoc::resolve_taxes_by_line_id(&state.db, &mach_tax_ids).await;
        grand_totals.push(taxed_quotation_grand_total(
            &mlines,
            &mlmach,
            &mat_taxes,
            &mach_taxes,
        ));
    }

    let page = InvoiceListPage {
        invoices,
        customer_names,
        grand_totals,
        path_and_query: path_and_query(&uri),
    };
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    if htmx.targets::<InvoiceTableKey>() {
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

pub async fn invoice_detail(
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(inv) = quotation::Entity::find_by_id(id)
        .one(&state.db)
        .await
        .unwrap_or(None)
    else {
        return Redirect::to(&WorkOrdersInvoicesRouteTag.url()).into_response();
    };

    let machine_lines_raw = quotation_machine_line::Entity::find()
        .filter(quotation_machine_line::Column::InvoiceId.eq(inv.id))
        .order_by_asc(quotation_machine_line::Column::Id)
        .all(&state.db)
        .await
        .unwrap_or_default();

    let material_lines_raw = quotation_material_line::Entity::find()
        .filter(quotation_material_line::Column::InvoiceId.eq(inv.id))
        .order_by_asc(quotation_material_line::Column::Id)
        .all(&state.db)
        .await
        .unwrap_or_default();

    let machine_ids: Vec<i64> = machine_lines_raw
        .iter()
        .filter_map(|l| l.machine_id)
        .collect();
    let machines = machine::Entity::find()
        .filter(machine::Column::Id.is_in(machine_ids))
        .all(&state.db)
        .await
        .unwrap_or_default();
    let machine_map: HashMap<i64, String> =
        machines.iter().map(|m| (m.id, m.name.clone())).collect();
    let machine_schemas: HashMap<i64, serde_json::Value> =
        machines.into_iter().map(|m| (m.id, m.variables)).collect();

    let comp_ids: Vec<i64> = material_lines_raw.iter().map(|l| l.component_id).collect();
    let comps = component::Entity::find()
        .filter(component::Column::Id.is_in(comp_ids))
        .all(&state.db)
        .await
        .unwrap_or_default();
    let comp_map: HashMap<i64, String> = comps.iter().map(|c| (c.id, c.name.clone())).collect();
    let component_schemas: HashMap<i64, serde_json::Value> =
        comps.into_iter().map(|c| (c.id, c.variables)).collect();

    let mat_ids: Vec<i64> = material_lines_raw.iter().map(|l| l.id).collect();
    let mach_ids: Vec<i64> = machine_lines_raw.iter().map(|l| l.id).collect();
    let mat_tax_ids = tax_assoc::load_quotation_material_line_tax_ids_map(&state.db, &mat_ids)
        .await
        .unwrap_or_default();
    let mach_tax_ids = tax_assoc::load_quotation_machine_line_tax_ids_map(&state.db, &mach_ids)
        .await
        .unwrap_or_default();
    let material_taxes = tax_assoc::resolve_taxes_by_line_id(&state.db, &mat_tax_ids).await;
    let machine_taxes = tax_assoc::resolve_taxes_by_line_id(&state.db, &mach_tax_ids).await;

    let material_lines: Vec<(quotation_material_line::Model, String, String, Decimal)> =
        material_lines_raw
            .iter()
            .cloned()
            .map(|l| {
                let c_name = comp_map
                    .get(&l.component_id)
                    .cloned()
                    .unwrap_or_else(|| format!("Component #{}", l.component_id));
                let taxes = tax_assoc::taxes_for_line(&material_taxes, l.id);
                let labels = tax_assoc::tax_labels_display(taxes);
                let taxed = l.taxed_total(taxes);
                (l, c_name, labels, taxed)
            })
            .collect();

    let machine_lines: Vec<(quotation_machine_line::Model, String, String, Decimal)> =
        machine_lines_raw
            .iter()
            .cloned()
            .map(|l| {
                let m_name = l
                    .machine_id
                    .and_then(|id| machine_map.get(&id).cloned())
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| l.name.clone());
                let taxes = tax_assoc::taxes_for_line(&machine_taxes, l.id);
                let labels = tax_assoc::tax_labels_display(taxes);
                let taxed = l.taxed_total(taxes);
                (l, m_name, labels, taxed)
            })
            .collect();

    let grand_total = taxed_quotation_grand_total(
        &material_lines_raw,
        &machine_lines_raw,
        &material_taxes,
        &machine_taxes,
    );

    let customer_name =
        lariv_rs::plugins::customer::entities::customer::Entity::find_by_id(inv.customer_id)
            .one(&state.db)
            .await
            .ok()
            .flatten()
            .map(|c| c.name)
            .unwrap_or_else(|| format!("Customer #{}", inv.customer_id));

    let page = InvoiceDetailPage {
        invoice: inv,
        machine_lines,
        material_lines,
        component_schemas,
        machine_schemas,
        grand_total,
        customer_name,
    };
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx).into_response()
}

async fn customer_name_by_id(db: &sea_orm::DatabaseConnection, id: i64) -> String {
    if id <= 0 {
        return String::new();
    }
    lariv_rs::plugins::customer::entities::customer::Entity::find_by_id(id)
        .one(db)
        .await
        .ok()
        .flatten()
        .map(|c| c.name)
        .unwrap_or_default()
}

/// Re-render the create modal with the submitted fields intact. HTMX `outerMorph`s
/// this over the open modal, so dropping any field here looks like a full reset.
async fn invoice_create_error_modal(
    db: &sea_orm::DatabaseConnection,
    form_name: String,
    form: &InvoiceFormData,
    error: String,
) -> InvoiceCreateModalPage {
    InvoiceCreateModalPage {
        form_name,
        invoice_number: form.invoice_number.clone(),
        date: form.date.clone(),
        customer_id: (form.customer_id > 0).then_some(form.customer_id),
        customer_name: customer_name_by_id(db, form.customer_id).await,
        material_lines_json: form.material_lines.clone().unwrap_or_default(),
        machine_lines_json: form.machine_lines.clone().unwrap_or_default(),
        components_json: components_json(db).await,
        machines_json: fetch_machines_json(db).await,
        taxes_json: tax_assoc::taxes_catalog_json(db).await,
        error,
    }
}

async fn invoice_edit_error_modal(
    db: &sea_orm::DatabaseConnection,
    id: i64,
    form_name: String,
    form: &InvoiceFormData,
    error: String,
) -> InvoiceEditModalPage {
    InvoiceEditModalPage {
        id,
        form_name,
        invoice_number: form.invoice_number.clone(),
        date: form.date.clone(),
        customer_id: form.customer_id,
        customer_name: customer_name_by_id(db, form.customer_id).await,
        material_lines_json: form.material_lines.clone().unwrap_or_default(),
        machine_lines_json: form.machine_lines.clone().unwrap_or_default(),
        components_json: components_json(db).await,
        machines_json: fetch_machines_json(db).await,
        taxes_json: tax_assoc::taxes_catalog_json(db).await,
        error,
    }
}

pub async fn invoice_create_get(
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    Query(q): Query<ModalFormQuery>,
) -> maud::Markup {
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let components_json = components_json(&state.db).await;
    let machines_json = fetch_machines_json(&state.db).await;

    let page = InvoiceCreateModalPage {
        form_name: q.form_name(),
        invoice_number: String::new(),
        date: today,
        customer_id: None,
        customer_name: String::new(),
        material_lines_json: "[]".into(),
        machine_lines_json: "[]".into(),
        components_json,
        machines_json,
        taxes_json: tax_assoc::taxes_catalog_json(&state.db).await,
        error: String::new(),
    };
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    html_built_page_with_slots(&page, &chrome, &slot_ctx)
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct InvoiceFormData {
    #[serde(alias = "invoice_number", default)]
    pub invoice_number: String,
    #[serde(alias = "date", default)]
    pub date: String,
    #[serde(
        alias = "customer_id",
        alias = "CustomerID",
        default,
        deserialize_with = "i64_from_str_or_zero"
    )]
    pub customer_id: i64,
    #[serde(alias = "material_lines", default)]
    pub material_lines: Option<String>,
    #[serde(alias = "machine_lines", default)]
    pub machine_lines: Option<String>,
}

pub async fn invoice_create_post(
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    htmx: Htmx,
    Query(q): Query<ModalFormQuery>,
    Form(form): Form<InvoiceFormData>,
) -> Response {
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();

    if form.customer_id <= 0 {
        let page = invoice_create_error_modal(
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
            let page = invoice_create_error_modal(
                &state.db,
                q.form_name(),
                &form,
                "Invalid date format. Use YYYY-MM-DD.".into(),
            )
            .await;
            return html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response();
        }
    };

    if let Err(e) = validate_machine_lines_json(form.machine_lines.as_deref()) {
        let page = invoice_create_error_modal(&state.db, q.form_name(), &form, e).await;
        return html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response();
    }

    let invoice_number =
        match quotation_number::resolve_quotation_number(&state.db, &form.invoice_number, date)
            .await
        {
            Ok(n) => n,
            Err(e) => {
                let page = invoice_create_error_modal(
                    &state.db,
                    q.form_name(),
                    &form,
                    format!("Failed to assign quotation number: {e}"),
                )
                .await;
                return html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response();
            }
        };

    let now = Utc::now();
    let model = quotation::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        date: Set(date),
        customer_id: Set(form.customer_id),
        invoice_number: Set(invoice_number),
    };

    match model.insert(&state.db).await {
        Ok(saved) => {
            sync_invoice_lines(
                &state.db,
                saved.id,
                form.material_lines,
                form.machine_lines,
                now,
            )
            .await;
            respond_create_modal_done::<InvoiceCreateModalKey>(
                &htmx,
                &q.refresh_table(),
                &InvoiceDetailRouteTag::new(saved.id).url(),
            )
        }
        Err(e) => {
            let page =
                invoice_create_error_modal(&state.db, q.form_name(), &form, e.to_string()).await;
            html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response()
        }
    }
}

pub async fn invoice_edit_get(
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    Query(q): Query<ModalFormQuery>,
    Path(id): Path<i64>,
) -> Response {
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    let inv = match quotation::Entity::find_by_id(id).one(&state.db).await {
        Ok(Some(i)) => i,
        _ => return Redirect::to(&WorkOrdersInvoicesRouteTag.url()).into_response(),
    };

    let customer_name =
        lariv_rs::plugins::customer::entities::customer::Entity::find_by_id(inv.customer_id)
            .one(&state.db)
            .await
            .ok()
            .flatten()
            .map(|c| c.name)
            .unwrap_or_else(|| format!("Customer #{}", inv.customer_id));

    let material_lines_json = invoice_material_lines_json(&state.db, inv.id).await;
    let machine_lines_json = invoice_machine_lines_json(&state.db, inv.id).await;
    let components_json = components_json(&state.db).await;
    let machines_json = fetch_machines_json(&state.db).await;

    let page = InvoiceEditModalPage {
        id,
        form_name: q.form_name(),
        invoice_number: inv.invoice_number,
        date: inv.date.to_string(),
        customer_id: inv.customer_id,
        customer_name,
        material_lines_json,
        machine_lines_json,
        components_json,
        machines_json,
        taxes_json: tax_assoc::taxes_catalog_json(&state.db).await,
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response()
}

pub async fn invoice_edit_post(
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    htmx: Htmx,
    Query(q): Query<ModalFormQuery>,
    Path(id): Path<i64>,
    Form(form): Form<InvoiceFormData>,
) -> impl IntoResponse {
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    let existing = match quotation::Entity::find_by_id(id).one(&state.db).await {
        Ok(Some(i)) => i,
        _ => return Redirect::to(&WorkOrdersInvoicesRouteTag.url()).into_response(),
    };

    let date =
        chrono::NaiveDate::parse_from_str(form.date.trim(), "%Y-%m-%d").unwrap_or(existing.date);

    if let Err(e) = validate_machine_lines_json(form.machine_lines.as_deref()) {
        let page = invoice_edit_error_modal(&state.db, id, q.form_name(), &form, e).await;
        return html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response();
    }

    let invoice_number =
        match quotation_number::resolve_quotation_number(&state.db, &form.invoice_number, date)
            .await
        {
            Ok(n) => n,
            Err(e) => {
                let page = invoice_edit_error_modal(
                    &state.db,
                    id,
                    q.form_name(),
                    &form,
                    format!("Failed to assign quotation number: {e}"),
                )
                .await;
                return html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response();
            }
        };

    let now = Utc::now();
    let mut am: quotation::ActiveModel = existing.into();
    am.updated_at = Set(Some(now));
    am.invoice_number = Set(invoice_number);
    am.date = Set(date);
    am.customer_id = Set(form.customer_id);

    match am.update(&state.db).await {
        Ok(_) => {
            sync_invoice_lines(&state.db, id, form.material_lines, form.machine_lines, now).await;
            respond_edit_modal_done::<InvoiceEditModalKey>(
                &htmx,
                &InvoiceDetailRouteTag::new(id).url(),
            )
        }
        Err(e) => {
            let page =
                invoice_edit_error_modal(&state.db, id, q.form_name(), &form, e.to_string()).await;
            html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response()
        }
    }
}

pub async fn invoice_delete_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    Path(id): Path<i64>,
) -> maud::Markup {
    let page = ConfirmDeleteModalPage {
        modal_uid: InvoiceDeleteModalKey::ID.to_string(),
        title: "Delete Quotation".into(),
        message: "Are you sure you want to delete this quotation?".into(),
        post_url: InvoiceDeletePostRouteTag::new(id).url(),
        error: String::new(),
    };
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    html_built_page_with_slots(&page, &chrome, &slot_ctx)
}

pub async fn invoice_delete_post(
    Cap(state): Cap<WorkOrdersState>,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let _ = quotation::Entity::delete_by_id(id).exec(&state.db).await;
    htmx.redirect(&WorkOrdersInvoicesRouteTag.url())
}

/// Copy a quotation into a final work order and schedule its machines as a job.
async fn create_work_order_from_quotation(
    db: &sea_orm::DatabaseConnection,
    quotation_id: i64,
    duration: JobDuration,
) -> Result<i64, String> {
    let inv = quotation::Entity::find_by_id(quotation_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Quotation not found".to_string())?;

    let material_lines = quotation_material_line::Entity::find()
        .filter(quotation_material_line::Column::InvoiceId.eq(inv.id))
        .order_by_asc(quotation_material_line::Column::Id)
        .all(db)
        .await
        .map_err(|e| e.to_string())?;
    let mat_ids: Vec<i64> = material_lines.iter().map(|l| l.id).collect();
    let mat_tax_map = tax_assoc::load_quotation_material_line_tax_ids_map(db, &mat_ids)
        .await
        .unwrap_or_default();

    let machine_lines = quotation_machine_line::Entity::find()
        .filter(quotation_machine_line::Column::InvoiceId.eq(inv.id))
        .order_by_asc(quotation_machine_line::Column::Id)
        .all(db)
        .await
        .map_err(|e| e.to_string())?;
    let mach_ids: Vec<i64> = machine_lines.iter().map(|l| l.id).collect();
    let mach_tax_map = tax_assoc::load_quotation_machine_line_tax_ids_map(db, &mach_ids)
        .await
        .unwrap_or_default();

    let mut machine_ids: Vec<i64> = Vec::new();
    for ql in &machine_lines {
        if let Some(machine_id) = ql.machine_id.filter(|id| *id > 0) {
            if !machine_ids.contains(&machine_id) {
                machine_ids.push(machine_id);
            }
        }
    }

    let now = Utc::now();
    let order_number = inv.invoice_number.clone();
    let job_name = if order_number.trim().is_empty() {
        format!("Work Order from {}", inv.invoice_number)
    } else {
        order_number.clone()
    };
    let remarks = format!("Work order from quotation {}", inv.invoice_number);

    let txn = db.begin().await.map_err(|e| e.to_string())?;
    let job = create_open_job(&txn, job_name, duration, &machine_ids, remarks).await?;

    let saved = work_order::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        order_number: Set(order_number),
        customer_id: Set(inv.customer_id),
        quotation_id: Set(Some(inv.id)),
        job_id: Set(Some(job.id)),
        duration: Set(duration),
    }
    .insert(&txn)
    .await
    .map_err(|e| e.to_string())?;

    set_job_source_doc(&txn, job.id, WORK_ORDER_SOURCE_DOC_TYPE, saved.id).await?;

    for ql in material_lines {
        let line = work_order_line::ActiveModel {
            id: Default::default(),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            work_order_id: Set(saved.id),
            component_id: Set(ql.component_id),
            variables: Set(ql.variables.clone()),
            final_cost: Set(ql.final_cost),
            extra_data: Set(ql.extra_data.clone()),
        }
        .insert(&txn)
        .await
        .map_err(|e| e.to_string())?;
        let tax_ids = mat_tax_map.get(&ql.id).cloned().unwrap_or_default();
        tax_assoc::set_work_order_material_line_taxes(&txn, line.id, &tax_ids)
            .await
            .map_err(|e| e.to_string())?;
    }

    for ql in machine_lines {
        let line = work_order_machine_line::ActiveModel {
            id: Default::default(),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            work_order_id: Set(saved.id),
            machine_id: Set(ql.machine_id.filter(|id| *id > 0)),
            name: Set(ql.name.clone()),
            variables: Set(ql.variables.clone()),
            final_cost: Set(ql.final_cost),
        }
        .insert(&txn)
        .await
        .map_err(|e| e.to_string())?;
        let tax_ids = mach_tax_map.get(&ql.id).cloned().unwrap_or_default();
        tax_assoc::set_work_order_machine_line_taxes(&txn, line.id, &tax_ids)
            .await
            .map_err(|e| e.to_string())?;
    }

    txn.commit().await.map_err(|e| e.to_string())?;
    Ok(saved.id)
}

pub async fn invoice_create_work_order_get(
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    Query(q): Query<ModalFormQuery>,
    Path(id): Path<i64>,
) -> Response {
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    let inv = match quotation::Entity::find_by_id(id).one(&state.db).await {
        Ok(Some(i)) => i,
        _ => return Redirect::to(&WorkOrdersInvoicesRouteTag.url()).into_response(),
    };

    let page = InvoiceCreateWorkOrderModalPage {
        id,
        form_name: q.form_name(),
        quotation_number: inv.invoice_number,
        duration: String::new(),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response()
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct InvoiceCreateWorkOrderFormData {
    #[serde(alias = "duration", default)]
    pub duration: String,
}

pub async fn invoice_create_work_order_post(
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    htmx: Htmx,
    Query(q): Query<ModalFormQuery>,
    Path(id): Path<i64>,
    Form(form): Form<InvoiceCreateWorkOrderFormData>,
) -> Response {
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    let inv = match quotation::Entity::find_by_id(id).one(&state.db).await {
        Ok(Some(i)) => i,
        _ => return Redirect::to(&WorkOrdersInvoicesRouteTag.url()).into_response(),
    };

    let duration = match parse_job_duration(&form.duration) {
        Ok(d) => d,
        Err(e) => {
            let page = InvoiceCreateWorkOrderModalPage {
                id,
                form_name: q.form_name(),
                quotation_number: inv.invoice_number,
                duration: form.duration.clone(),
                error: format!("Invalid duration: {e}"),
            };
            return html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response();
        }
    };

    match create_work_order_from_quotation(&state.db, id, duration).await {
        Ok(order_id) => respond_create_modal_done::<InvoiceCreateWorkOrderModalKey>(
            &htmx,
            &q.refresh_table(),
            &IssuedWorkOrderDetailRouteTag::new(order_id).url(),
        ),
        Err(e) => {
            let page = InvoiceCreateWorkOrderModalPage {
                id,
                form_name: q.form_name(),
                quotation_number: inv.invoice_number,
                duration: form.duration,
                error: e,
            };
            html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response()
        }
    }
}

/// Convert a draft work order into a work order, then delete the draft.
async fn convert_draft_to_work_order(
    db: &sea_orm::DatabaseConnection,
    draft_id: i64,
) -> Result<i64, String> {
    let draft = draft_work_order::Entity::find_by_id(draft_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Draft work order not found".to_string())?;

    let material_lines = draft_work_order_material_line::Entity::find()
        .filter(draft_work_order_material_line::Column::DraftWorkOrderId.eq(draft.id))
        .order_by_asc(draft_work_order_material_line::Column::Id)
        .all(db)
        .await
        .map_err(|e| e.to_string())?;
    let mat_ids: Vec<i64> = material_lines.iter().map(|l| l.id).collect();
    let mat_tax_map = tax_assoc::load_draft_material_line_tax_ids_map(db, &mat_ids)
        .await
        .unwrap_or_default();

    let machine_lines = draft_work_order_machine_line::Entity::find()
        .filter(draft_work_order_machine_line::Column::DraftWorkOrderId.eq(draft.id))
        .order_by_asc(draft_work_order_machine_line::Column::Id)
        .all(db)
        .await
        .map_err(|e| e.to_string())?;
    let mach_ids: Vec<i64> = machine_lines.iter().map(|l| l.id).collect();
    let mach_tax_map = tax_assoc::load_draft_machine_line_tax_ids_map(db, &mach_ids)
        .await
        .unwrap_or_default();

    let mut machine_ids: Vec<i64> = Vec::new();
    for line in &machine_lines {
        if line.machine_id > 0 && !machine_ids.contains(&line.machine_id) {
            machine_ids.push(line.machine_id);
        }
    }

    let machines = machine::Entity::find()
        .filter(machine::Column::Id.is_in(machine_ids.clone()))
        .all(db)
        .await
        .map_err(|e| e.to_string())?;
    let machine_names: HashMap<i64, String> =
        machines.into_iter().map(|m| (m.id, m.name)).collect();

    let now = Utc::now();
    let order_number = draft.order_number.clone();
    let job_name = if order_number.trim().is_empty() {
        format!("Work Order #{draft_id}")
    } else {
        order_number.clone()
    };
    let remarks = if order_number.trim().is_empty() {
        "Work order from draft".to_string()
    } else {
        format!("Work order from draft {order_number}")
    };

    let txn = db.begin().await.map_err(|e| e.to_string())?;
    let job = create_open_job(&txn, job_name, draft.duration, &machine_ids, remarks).await?;

    let saved = work_order::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        order_number: Set(order_number),
        customer_id: Set(draft.customer_id),
        quotation_id: Set(draft.quotation_id),
        job_id: Set(Some(job.id)),
        duration: Set(draft.duration),
    }
    .insert(&txn)
    .await
    .map_err(|e| e.to_string())?;

    set_job_source_doc(&txn, job.id, WORK_ORDER_SOURCE_DOC_TYPE, saved.id).await?;

    for line in material_lines {
        let copied = work_order_line::ActiveModel {
            id: Default::default(),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            work_order_id: Set(saved.id),
            component_id: Set(line.component_id),
            variables: Set(line.variables.clone()),
            final_cost: Set(line.final_cost),
            extra_data: Set(line.extra_data.clone()),
        }
        .insert(&txn)
        .await
        .map_err(|e| e.to_string())?;
        let tax_ids = mat_tax_map.get(&line.id).cloned().unwrap_or_default();
        tax_assoc::set_work_order_material_line_taxes(&txn, copied.id, &tax_ids)
            .await
            .map_err(|e| e.to_string())?;
    }

    for line in machine_lines {
        let name = machine_names
            .get(&line.machine_id)
            .cloned()
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| format!("Machine #{}", line.machine_id));
        let copied = work_order_machine_line::ActiveModel {
            id: Default::default(),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            work_order_id: Set(saved.id),
            machine_id: Set(if line.machine_id > 0 {
                Some(line.machine_id)
            } else {
                None
            }),
            name: Set(name),
            variables: Set(line.variables.clone()),
            final_cost: Set(line.final_cost),
        }
        .insert(&txn)
        .await
        .map_err(|e| e.to_string())?;
        let tax_ids = mach_tax_map.get(&line.id).cloned().unwrap_or_default();
        tax_assoc::set_work_order_machine_line_taxes(&txn, copied.id, &tax_ids)
            .await
            .map_err(|e| e.to_string())?;
    }

    draft_work_order::Entity::delete_by_id(draft.id)
        .exec(&txn)
        .await
        .map_err(|e| e.to_string())?;

    txn.commit().await.map_err(|e| e.to_string())?;
    Ok(saved.id)
}

pub async fn work_order_convert_post(
    Cap(state): Cap<WorkOrdersState>,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    match convert_draft_to_work_order(&state.db, id).await {
        Ok(order_id) => htmx.redirect(&IssuedWorkOrderDetailRouteTag::new(order_id).url()),
        Err(_) => htmx.redirect(&WorkOrderDetailRouteTag::new(id).url()),
    }
}

/// Copy a work order into a new draft without deleting the work order.
async fn new_draft_from_work_order(
    db: &sea_orm::DatabaseConnection,
    work_order_id: i64,
) -> Result<i64, String> {
    let order = work_order::Entity::find_by_id(work_order_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Work order not found".to_string())?;

    let material_lines = work_order_line::Entity::find()
        .filter(work_order_line::Column::WorkOrderId.eq(order.id))
        .order_by_asc(work_order_line::Column::Id)
        .all(db)
        .await
        .map_err(|e| e.to_string())?;
    let mat_ids: Vec<i64> = material_lines.iter().map(|l| l.id).collect();
    let mat_tax_map = tax_assoc::load_work_order_material_line_tax_ids_map(db, &mat_ids)
        .await
        .unwrap_or_default();

    let machine_lines = work_order_machine_line::Entity::find()
        .filter(work_order_machine_line::Column::WorkOrderId.eq(order.id))
        .order_by_asc(work_order_machine_line::Column::Id)
        .all(db)
        .await
        .map_err(|e| e.to_string())?;
    let mach_ids: Vec<i64> = machine_lines.iter().map(|l| l.id).collect();
    let mach_tax_map = tax_assoc::load_work_order_machine_line_tax_ids_map(db, &mach_ids)
        .await
        .unwrap_or_default();

    let now = Utc::now();
    let txn = db.begin().await.map_err(|e| e.to_string())?;

    let saved = draft_work_order::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        order_number: Set(order.order_number.clone()),
        customer_id: Set(order.customer_id),
        quotation_id: Set(order.quotation_id),
        duration: Set(order.duration),
    }
    .insert(&txn)
    .await
    .map_err(|e| e.to_string())?;

    for line in material_lines {
        let copied = draft_work_order_material_line::ActiveModel {
            id: Default::default(),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            draft_work_order_id: Set(saved.id),
            component_id: Set(line.component_id),
            variables: Set(line.variables.clone()),
            final_cost: Set(line.final_cost),
            extra_data: Set(line.extra_data.clone()),
        }
        .insert(&txn)
        .await
        .map_err(|e| e.to_string())?;
        let tax_ids = mat_tax_map.get(&line.id).cloned().unwrap_or_default();
        tax_assoc::set_draft_material_line_taxes(&txn, copied.id, &tax_ids)
            .await
            .map_err(|e| e.to_string())?;
    }

    for line in machine_lines {
        let Some(machine_id) = line.machine_id.filter(|id| *id > 0) else {
            continue;
        };
        let copied = draft_work_order_machine_line::ActiveModel {
            id: Default::default(),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            draft_work_order_id: Set(saved.id),
            machine_id: Set(machine_id),
            variables: Set(line.variables.clone()),
            final_cost: Set(line.final_cost),
        }
        .insert(&txn)
        .await
        .map_err(|e| e.to_string())?;
        let tax_ids = mach_tax_map.get(&line.id).cloned().unwrap_or_default();
        tax_assoc::set_draft_machine_line_taxes(&txn, copied.id, &tax_ids)
            .await
            .map_err(|e| e.to_string())?;
    }

    txn.commit().await.map_err(|e| e.to_string())?;
    Ok(saved.id)
}

pub async fn issued_work_order_new_draft_post(
    Cap(state): Cap<WorkOrdersState>,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    match new_draft_from_work_order(&state.db, id).await {
        Ok(draft_id) => htmx.redirect(&WorkOrderDetailRouteTag::new(draft_id).url()),
        Err(_) => htmx.redirect(&IssuedWorkOrderDetailRouteTag::new(id).url()),
    }
}

fn taxed_work_order_grand_total(
    lines: &[work_order_line::Model],
    machine_lines: &[work_order_machine_line::Model],
    material_taxes: &HashMap<i64, Vec<lariv_rs::plugins::finance_taxes::entities::tax::Model>>,
    machine_taxes: &HashMap<i64, Vec<lariv_rs::plugins::finance_taxes::entities::tax::Model>>,
) -> Decimal {
    let materials: Decimal = lines
        .iter()
        .map(|l| l.taxed_total(tax_assoc::taxes_for_line(material_taxes, l.id)))
        .sum();
    let machines: Decimal = machine_lines
        .iter()
        .map(|l| l.taxed_total(tax_assoc::taxes_for_line(machine_taxes, l.id)))
        .sum();
    materials + machines
}

pub async fn issued_work_orders_list(
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    htmx: Htmx,
    uri: Uri,
) -> maud::Markup {
    let orders = work_order::Entity::find()
        .order_by_desc(work_order::Column::Id)
        .all(&state.db)
        .await
        .unwrap_or_default();

    let all_lines = work_order_line::Entity::find()
        .all(&state.db)
        .await
        .unwrap_or_default();
    let all_machine_lines = work_order_machine_line::Entity::find()
        .all(&state.db)
        .await
        .unwrap_or_default();

    let mut lines_by_order: HashMap<i64, Vec<work_order_line::Model>> = HashMap::new();
    for line in all_lines {
        lines_by_order
            .entry(line.work_order_id)
            .or_default()
            .push(line);
    }
    let mut machine_lines_by_order: HashMap<i64, Vec<work_order_machine_line::Model>> =
        HashMap::new();
    for line in all_machine_lines {
        machine_lines_by_order
            .entry(line.work_order_id)
            .or_default()
            .push(line);
    }

    let material_ids: Vec<i64> = lines_by_order.values().flatten().map(|l| l.id).collect();
    let machine_ids: Vec<i64> = machine_lines_by_order
        .values()
        .flatten()
        .map(|l| l.id)
        .collect();
    let material_tax_ids =
        tax_assoc::load_work_order_material_line_tax_ids_map(&state.db, &material_ids)
            .await
            .unwrap_or_default();
    let machine_tax_ids =
        tax_assoc::load_work_order_machine_line_tax_ids_map(&state.db, &machine_ids)
            .await
            .unwrap_or_default();
    let material_taxes = tax_assoc::resolve_taxes_by_line_id(&state.db, &material_tax_ids).await;
    let machine_taxes = tax_assoc::resolve_taxes_by_line_id(&state.db, &machine_tax_ids).await;

    let mut customer_names = Vec::with_capacity(orders.len());
    let mut job_labels = Vec::with_capacity(orders.len());
    let mut totals = Vec::with_capacity(orders.len());
    let mut line_counts = Vec::with_capacity(orders.len());
    for o in &orders {
        let name =
            lariv_rs::plugins::customer::entities::customer::Entity::find_by_id(o.customer_id)
                .one(&state.db)
                .await
                .ok()
                .flatten()
                .map(|c| c.name)
                .unwrap_or_else(|| format!("Customer #{}", o.customer_id));
        customer_names.push(name);
        job_labels.push(match o.job_id {
            Some(id) => format!("#{id}"),
            None => "—".into(),
        });
        let lines = lines_by_order
            .get(&o.id)
            .map(|v| v.as_slice())
            .unwrap_or(&[]);
        let machine_lines = machine_lines_by_order
            .get(&o.id)
            .map(|v| v.as_slice())
            .unwrap_or(&[]);
        totals.push(taxed_work_order_grand_total(
            lines,
            machine_lines,
            &material_taxes,
            &machine_taxes,
        ));
        line_counts.push(lines.len() + machine_lines.len());
    }

    let page = IssuedWorkOrderListPage {
        orders,
        customer_names,
        job_labels,
        line_counts,
        totals,
        path_and_query: path_and_query(&uri),
    };
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    if htmx.targets::<IssuedWorkOrderTableKey>() {
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

pub async fn issued_work_order_detail(
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(order) = work_order::Entity::find_by_id(id)
        .one(&state.db)
        .await
        .unwrap_or(None)
    else {
        return Redirect::to(&IssuedWorkOrdersRouteTag.url()).into_response();
    };

    let lines = work_order_line::Entity::find()
        .filter(work_order_line::Column::WorkOrderId.eq(id))
        .order_by_asc(work_order_line::Column::Id)
        .all(&state.db)
        .await
        .unwrap_or_default();
    let machine_lines = work_order_machine_line::Entity::find()
        .filter(work_order_machine_line::Column::WorkOrderId.eq(id))
        .order_by_asc(work_order_machine_line::Column::Id)
        .all(&state.db)
        .await
        .unwrap_or_default();

    let machine_ids: Vec<i64> = machine_lines.iter().filter_map(|l| l.machine_id).collect();
    let machines = machine::Entity::find()
        .filter(machine::Column::Id.is_in(machine_ids))
        .all(&state.db)
        .await
        .unwrap_or_default();
    let machine_map: HashMap<i64, String> =
        machines.iter().map(|m| (m.id, m.name.clone())).collect();
    let machine_schemas: HashMap<i64, serde_json::Value> =
        machines.into_iter().map(|m| (m.id, m.variables)).collect();

    let comp_ids: Vec<i64> = lines.iter().map(|l| l.component_id).collect();
    let comps = component::Entity::find()
        .filter(component::Column::Id.is_in(comp_ids))
        .all(&state.db)
        .await
        .unwrap_or_default();
    let comp_map: HashMap<i64, String> = comps.iter().map(|c| (c.id, c.name.clone())).collect();
    let component_schemas: HashMap<i64, serde_json::Value> =
        comps.into_iter().map(|c| (c.id, c.variables)).collect();

    let customer =
        lariv_rs::plugins::customer::entities::customer::Entity::find_by_id(order.customer_id)
            .one(&state.db)
            .await
            .ok()
            .flatten();

    let quotation_number = if let Some(qid) = order.quotation_id {
        quotation::Entity::find_by_id(qid)
            .one(&state.db)
            .await
            .ok()
            .flatten()
            .map(|q| q.invoice_number)
    } else {
        None
    };

    let job_name = if let Some(job_id) = order.job_id {
        job::Entity::find_by_id(job_id)
            .one(&state.db)
            .await
            .ok()
            .flatten()
            .map(|j| j.name)
    } else {
        None
    };

    let material_ids: Vec<i64> = lines.iter().map(|l| l.id).collect();
    let machine_line_ids: Vec<i64> = machine_lines.iter().map(|l| l.id).collect();
    let material_tax_ids =
        tax_assoc::load_work_order_material_line_tax_ids_map(&state.db, &material_ids)
            .await
            .unwrap_or_default();
    let machine_tax_ids =
        tax_assoc::load_work_order_machine_line_tax_ids_map(&state.db, &machine_line_ids)
            .await
            .unwrap_or_default();
    let material_taxes = tax_assoc::resolve_taxes_by_line_id(&state.db, &material_tax_ids).await;
    let machine_taxes = tax_assoc::resolve_taxes_by_line_id(&state.db, &machine_tax_ids).await;

    let total_amount =
        taxed_work_order_grand_total(&lines, &machine_lines, &material_taxes, &machine_taxes);

    let lines_with_comp: Vec<(work_order_line::Model, String, String, Decimal)> = lines
        .into_iter()
        .map(|l| {
            let c_name = comp_map
                .get(&l.component_id)
                .cloned()
                .unwrap_or_else(|| format!("Component #{}", l.component_id));
            let taxes = tax_assoc::taxes_for_line(&material_taxes, l.id);
            let labels = tax_assoc::tax_labels_display(taxes);
            let taxed = l.taxed_total(taxes);
            (l, c_name, labels, taxed)
        })
        .collect();

    let machine_lines_with_name: Vec<(work_order_machine_line::Model, String, String, Decimal)> =
        machine_lines
            .into_iter()
            .map(|l| {
                let m_name = l
                    .machine_id
                    .and_then(|id| machine_map.get(&id).cloned())
                    .filter(|n| !n.is_empty())
                    .unwrap_or_else(|| {
                        if l.name.trim().is_empty() {
                            l.machine_id
                                .map(|id| format!("Machine #{id}"))
                                .unwrap_or_else(|| "Machine".into())
                        } else {
                            l.name.clone()
                        }
                    });
                let taxes = tax_assoc::taxes_for_line(&machine_taxes, l.id);
                let labels = tax_assoc::tax_labels_display(taxes);
                let taxed = l.taxed_total(taxes);
                (l, m_name, labels, taxed)
            })
            .collect();

    let page = IssuedWorkOrderDetailPage {
        order,
        lines: lines_with_comp,
        machine_lines: machine_lines_with_name,
        component_schemas,
        machine_schemas,
        customer_name: customer.map(|c| c.name),
        quotation_number,
        job_name,
        total_amount,
    };
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx).into_response()
}

pub async fn issued_work_order_delete_get(
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    Path(id): Path<i64>,
) -> maud::Markup {
    let page = build_issued_work_order_delete_modal(&state.db, id, None).await;
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    html_built_page_with_slots(&page, &chrome, &slot_ctx)
}

pub async fn issued_work_order_delete_post(
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    match delete_work_order_recursive(&state.db, id).await {
        Ok(()) => htmx.redirect(&IssuedWorkOrdersRouteTag.url()),
        Err(e) => {
            tracing::error!(error = %e, id, "failed to delete work order");
            let page =
                build_issued_work_order_delete_modal(&state.db, id, Some(e.to_string())).await;
            let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
            html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response()
        }
    }
}

async fn build_issued_work_order_delete_modal(
    db: &sea_orm::DatabaseConnection,
    id: i64,
    error: Option<String>,
) -> IssuedWorkOrderDeleteModalPage {
    let (items, collect_error) = match collect_work_order_cascade(db, id).await {
        Ok(graph) => match cascade_delete_preview(db, &graph).await {
            Ok(items) => (
                items
                    .into_iter()
                    .map(|item| IssuedWorkOrderDeleteItem {
                        kind: item.kind,
                        label: item.label,
                        url: item.url,
                    })
                    .collect(),
                None,
            ),
            Err(e) => (Vec::new(), Some(e.to_string())),
        },
        Err(e) => (Vec::new(), Some(e.to_string())),
    };
    IssuedWorkOrderDeleteModalPage {
        id,
        items,
        can_delete: collect_error.is_none(),
        error: collect_error.or(error),
    }
}

// ==========================================
// 8. FORMULA CALCULATE API
// ==========================================

#[derive(Debug, Deserialize)]
pub struct CalculateRequest {
    #[serde(default)]
    pub component_id: Option<i64>,
    #[serde(default)]
    pub machine_id: Option<i64>,
    #[serde(default)]
    pub variables: serde_json::Value,
    #[serde(default)]
    pub extra_data: Option<serde_json::Value>,
}

pub async fn calculate_api(
    Cap(state): Cap<WorkOrdersState>,
    Json(payload): Json<CalculateRequest>,
) -> impl IntoResponse {
    let raw_vars = variables_from_value(Some(&payload.variables));
    if let Some(component_id) = payload.component_id.filter(|id| *id > 0) {
        let extra = extra_data_from_value(payload.extra_data.as_ref());
        let units = line_vars::dim_units_map(&extra);
        let comp = match component::Entity::find_by_id(component_id)
            .one(&state.db)
            .await
        {
            Ok(Some(c)) => c,
            Ok(None) => {
                return Json(serde_json::json!({
                    "error": format!("Component #{component_id} not found")
                }));
            }
            Err(e) => {
                return Json(serde_json::json!({ "error": e.to_string() }));
            }
        };
        let schema = match comp.variables_schema() {
            Ok(s) => s,
            Err(e) => return Json(serde_json::json!({ "error": e.to_string() })),
        };
        let values = match parse_values_from_json(&schema, &raw_vars, &units) {
            Ok(v) => v,
            Err(e) => return Json(serde_json::json!({ "error": e.to_string() })),
        };
        let weight = match comp.get_weight(&values) {
            Ok(w) => w,
            Err(e) => return Json(serde_json::json!({ "error": e.to_string() })),
        };
        let cost = match comp.get_cost(&values) {
            Ok(c) => c,
            Err(e) => return Json(serde_json::json!({ "error": e.to_string() })),
        };
        return Json(serde_json::json!({ "weight": weight, "cost": cost }));
    }

    if let Some(machine_id) = payload.machine_id.filter(|id| *id > 0) {
        let mach = match machine::Entity::find_by_id(machine_id).one(&state.db).await {
            Ok(Some(m)) => m,
            Ok(None) => {
                return Json(serde_json::json!({
                    "error": format!("Machine #{machine_id} not found")
                }));
            }
            Err(e) => {
                return Json(serde_json::json!({ "error": e.to_string() }));
            }
        };
        let schema = match mach.variables_schema() {
            Ok(s) => s,
            Err(e) => return Json(serde_json::json!({ "error": e.to_string() })),
        };
        let values = match parse_values_from_json(&schema, &raw_vars, &HashMap::new()) {
            Ok(v) => v,
            Err(e) => return Json(serde_json::json!({ "error": e.to_string() })),
        };
        let cost = match mach.get_cost(&values) {
            Ok(c) => c,
            Err(e) => return Json(serde_json::json!({ "error": e.to_string() })),
        };
        return Json(serde_json::json!({ "cost": cost }));
    }

    Json(serde_json::json!({
        "error": "component_id or machine_id is required"
    }))
}

// ==========================================
// 10. PDF EXPORT + PREFERENCES
// ==========================================

const WORK_ORDER_PDF_PREVIEW_CACHE_DIR: &str = "lariv-work-orders-pdf-preview";

fn preview_cache_dir() -> PathBuf {
    std::env::temp_dir().join(WORK_ORDER_PDF_PREVIEW_CACHE_DIR)
}

fn preview_token() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{}-{}", std::process::id(), nanos)
}

fn is_valid_preview_token(token: &str) -> bool {
    !token.is_empty()
        && token.len() <= 64
        && token
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn preview_pdf_path(token: &str) -> PathBuf {
    preview_cache_dir().join(format!("{token}.pdf"))
}

fn store_preview_pdf(token: &str, bytes: &[u8]) -> Result<(), String> {
    let dir = preview_cache_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("create preview cache: {e}"))?;
    std::fs::write(preview_pdf_path(token), bytes).map_err(|e| format!("write preview pdf: {e}"))
}

fn remove_preview_pdf(token: &str) {
    let path = preview_pdf_path(token);
    if let Err(e) = std::fs::remove_file(&path) {
        if e.kind() != std::io::ErrorKind::NotFound {
            tracing::warn!(error = %e, path = %path.display(), "failed to remove preview pdf");
        }
    }
}

fn cleanup_stale_previews(max_age_secs: u64) {
    let Ok(read_dir) = std::fs::read_dir(preview_cache_dir()) else {
        return;
    };
    let cutoff = std::time::SystemTime::now()
        .checked_sub(std::time::Duration::from_secs(max_age_secs))
        .unwrap_or(std::time::UNIX_EPOCH);
    for entry in read_dir.flatten() {
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        let Ok(modified) = meta.modified() else {
            continue;
        };
        if modified < cutoff {
            if let Err(e) = std::fs::remove_file(entry.path()) {
                if e.kind() != std::io::ErrorKind::NotFound {
                    tracing::warn!(
                        error = %e,
                        path = %entry.path().display(),
                        "failed to remove stale preview pdf"
                    );
                }
            }
        }
    }
}

fn pdf_error_response(err: PdfError) -> Response {
    match err {
        PdfError::NotFound => (StatusCode::NOT_FOUND, "Not found").into_response(),
        PdfError::Message(msg) => {
            tracing::error!("kds quotations pdf: {msg}");
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
    modal_keyed::<WorkOrdersPdfModalKey>(
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
    modal_keyed::<WorkOrdersPdfModalKey>(
        "max-w-2xl",
        html! {
            h3 class="text-lg font-semibold mb-2" { "PDF preview failed" }
            p class="text-error whitespace-pre-wrap" { (message) }
        },
    )
}

fn render_preview_modal(pdf_url: &str, error: Option<&str>) -> Markup {
    if let Some(err) = error {
        return modal_keyed::<WorkOrdersPdfPreviewModalKey>(
            "max-w-2xl",
            html! {
                h3 class="text-lg font-semibold mb-2" { "KDS Quotations PDF preview failed" }
                p class="text-error whitespace-pre-wrap" { (err) }
            },
        );
    }
    modal_keyed::<WorkOrdersPdfPreviewModalKey>(
        "max-w-6xl w-[95vw]",
        html! {
            h3 class="text-lg font-semibold mb-3" { "KDS Quotations PDF preview (sample data)" }
            iframe
                src=(pdf_url)
                class="w-full h-[75vh] border border-base-300 rounded bg-white"
                title="KDS Quotations PDF preview" {}
        },
    )
}

async fn prefs_page(
    db: &sea_orm::DatabaseConnection,
    prefs: WorkOrdersPreferences,
    default_material_tax_ids: Option<&[i64]>,
    default_machine_tax_ids: Option<&[i64]>,
    error: String,
) -> WorkOrdersPreferencesPage {
    let mat_ids = match default_material_tax_ids {
        Some(ids) => ids.to_vec(),
        None => tax_assoc::load_default_material_tax_ids(db)
            .await
            .unwrap_or_default(),
    };
    let mach_ids = match default_machine_tax_ids {
        Some(ids) => ids.to_vec(),
        None => tax_assoc::load_default_machine_tax_ids(db)
            .await
            .unwrap_or_default(),
    };
    WorkOrdersPreferencesPage {
        draft_work_order_pdf_template: draft_work_order_pdf_template(&prefs).to_string(),
        work_order_pdf_template: work_order_pdf_template(&prefs).to_string(),
        quotation_pdf_template: quotation_pdf_template(&prefs).to_string(),
        quotation_number_format: prefs.quotation_number_format.unwrap_or_default(),
        company_name: prefs.company_name.unwrap_or_default(),
        company_address: prefs.company_address.unwrap_or_default(),
        company_phone: prefs.company_phone.unwrap_or_default(),
        company_gstin: prefs.company_gstin.unwrap_or_default(),
        place_of_supply: prefs.place_of_supply.unwrap_or_default(),
        company_logo_vnode_id: fk_value(prefs.company_logo_vnode_id),
        company_logo_vnode_display: load_vnode_display(db, prefs.company_logo_vnode_id).await,
        company_signature_vnode_id: fk_value(prefs.company_signature_vnode_id),
        company_signature_vnode_display: load_vnode_display(db, prefs.company_signature_vnode_id)
            .await,
        default_material_tax_items: tax_items_for_ids(db, &mat_ids).await,
        default_machine_tax_items: tax_items_for_ids(db, &mach_ids).await,
        error,
    }
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

/// HTTP handler: `get /work-orders/preferences`.
pub async fn preferences_get(
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireStaff(ctx): RequireStaff,
    htmx: Htmx,
) -> Response {
    let slot_ctx = SlotCtx::from_auth(&ctx);
    let prefs = match load_preferences(&state.db).await {
        Ok(p) => p,
        Err(e) => {
            let page = prefs_page(
                &state.db,
                empty_preferences(),
                None,
                None,
                format!("Failed to load preferences: {e}"),
            )
            .await;
            return html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx).into_response();
        }
    };
    let page = prefs_page(&state.db, prefs, None, None, String::new()).await;
    html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx).into_response()
}

/// HTTP handler: `post /work-orders/preferences`.
pub async fn preferences_post(
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireStaff(ctx): RequireStaff,
    htmx: Htmx,
    HtmlFormBody(form): HtmlFormBody<WorkOrdersPreferencesForm>,
) -> Response {
    let slot_ctx = SlotCtx::from_auth(&ctx);
    let quotation_number_format = {
        let t = form.quotation_number_format.trim();
        if t.is_empty() {
            None
        } else {
            Some(t.to_string())
        }
    };
    let prefs = WorkOrdersPreferences {
        id: 1,
        created_at: None,
        updated_at: None,
        draft_work_order_pdf_template: if is_stock_draft_work_order_template(Some(
            form.draft_work_order_pdf_template.as_str(),
        )) {
            None
        } else {
            Some(form.draft_work_order_pdf_template)
        },
        work_order_pdf_template: if is_stock_work_order_template(Some(
            form.work_order_pdf_template.as_str(),
        )) {
            None
        } else {
            Some(form.work_order_pdf_template)
        },
        quotation_pdf_template: if is_stock_quotation_template(Some(
            form.quotation_pdf_template.as_str(),
        )) {
            None
        } else {
            Some(form.quotation_pdf_template)
        },
        quotation_number_format,
        company_name: opt_text(&form.company_name),
        company_address: opt_text(&form.company_address),
        company_phone: opt_text(&form.company_phone),
        company_gstin: opt_text(&form.company_gstin),
        place_of_supply: opt_text(&form.place_of_supply),
        company_logo_vnode_id: opt_vnode_id(&form.company_logo_vnode_id),
        company_signature_vnode_id: opt_vnode_id(&form.company_signature_vnode_id),
    };
    match save_preferences(&state.db, prefs.clone()).await {
        Ok(_) => {
            let _ = tax_assoc::set_default_material_taxes(
                &state.db,
                &tax_assoc::normalize_tax_ids(&form.default_material_taxes),
            )
            .await;
            let _ = tax_assoc::set_default_machine_taxes(
                &state.db,
                &tax_assoc::normalize_tax_ids(&form.default_machine_taxes),
            )
            .await;
            htmx.redirect(&WorkOrdersPrefsGetRouteTag.url())
        }
        Err(e) => {
            let page = prefs_page(
                &state.db,
                prefs,
                Some(&form.default_material_taxes),
                Some(&form.default_machine_taxes),
                format!("Failed to save preferences: {e}"),
            )
            .await;
            html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx).into_response()
        }
    }
}

/// HTTP handler: `get /work-orders/orders/{id}/pdf`.
pub async fn work_order_pdf_modal(
    Cap(state): Cap<WorkOrdersState>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
) -> Markup {
    if !require_superuser(&ctx) {
        return render_pdf_modal_error("Forbidden");
    }
    let Some(order) = draft_work_order::Entity::find_by_id(id)
        .one(&state.db)
        .await
        .unwrap_or(None)
    else {
        return render_pdf_modal_error("Draft work order not found");
    };
    let title = if order.order_number.trim().is_empty() {
        "Draft Work Order PDF".to_string()
    } else {
        format!("Draft Work Order {} PDF", order.order_number)
    };
    render_pdf_modal(&title, &WorkOrderPdfRouteTag::new(id).path())
}

/// HTTP handler: `get /work-orders/orders/{id}/pdf/file`.
pub async fn work_order_pdf(
    Cap(state): Cap<WorkOrdersState>,
    Cap(fs): Cap<FilesystemState>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
) -> Response {
    if !require_superuser(&ctx) {
        return StatusCode::FORBIDDEN.into_response();
    }
    match pdf::render_work_order_pdf(&state.db, Some(&fs), id, &ctx.timezone).await {
        Ok(result) => pdf_ok_response(result),
        Err(e) => pdf_error_response(e),
    }
}

/// HTTP handler: `get /work-orders/issued/{id}/pdf`.
pub async fn issued_work_order_pdf_modal(
    Cap(state): Cap<WorkOrdersState>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
) -> Markup {
    if !require_superuser(&ctx) {
        return render_pdf_modal_error("Forbidden");
    }
    let Some(order) = work_order::Entity::find_by_id(id)
        .one(&state.db)
        .await
        .unwrap_or(None)
    else {
        return render_pdf_modal_error("Work order not found");
    };
    let title = if order.order_number.trim().is_empty() {
        "Work Order PDF".to_string()
    } else {
        format!("Work Order {} PDF", order.order_number)
    };
    render_pdf_modal(&title, &IssuedWorkOrderPdfRouteTag::new(id).path())
}

/// HTTP handler: `get /work-orders/issued/{id}/pdf/file`.
pub async fn issued_work_order_pdf(
    Cap(state): Cap<WorkOrdersState>,
    Cap(fs): Cap<FilesystemState>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
) -> Response {
    if !require_superuser(&ctx) {
        return StatusCode::FORBIDDEN.into_response();
    }
    match pdf::render_issued_work_order_pdf(&state.db, Some(&fs), id, &ctx.timezone).await {
        Ok(result) => pdf_ok_response(result),
        Err(e) => pdf_error_response(e),
    }
}

/// HTTP handler: `get /work-orders/quotations/{id}/pdf`.
pub async fn invoice_pdf_modal(
    Cap(state): Cap<WorkOrdersState>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
) -> Markup {
    if !require_superuser(&ctx) {
        return render_pdf_modal_error("Forbidden");
    }
    let Some(inv) = quotation::Entity::find_by_id(id)
        .one(&state.db)
        .await
        .unwrap_or(None)
    else {
        return render_pdf_modal_error("Quotation not found");
    };
    let title = if inv.invoice_number.trim().is_empty() {
        "Quotation PDF".to_string()
    } else {
        format!("Quotation {} PDF", inv.invoice_number)
    };
    render_pdf_modal(&title, &InvoicePdfRouteTag::new(id).path())
}

/// HTTP handler: `get /work-orders/quotations/{id}/pdf/file`.
pub async fn invoice_pdf(
    Cap(state): Cap<WorkOrdersState>,
    Cap(fs): Cap<FilesystemState>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
) -> Response {
    if !require_superuser(&ctx) {
        return StatusCode::FORBIDDEN.into_response();
    }
    match pdf::render_quotation_pdf(&state.db, Some(&fs), id, &ctx.timezone).await {
        Ok(result) => pdf_ok_response(result),
        Err(e) => pdf_error_response(e),
    }
}

fn company_presentation_from_form(form: &WorkOrdersPreferencesForm) -> pdf::CompanyPresentation {
    pdf::CompanyPresentation {
        name: form.company_name.clone(),
        address: form.company_address.clone(),
        phone: form.company_phone.clone(),
        gstin: form.company_gstin.clone(),
        place_of_supply: form.place_of_supply.clone(),
        logo_vnode_id: opt_vnode_id(&form.company_logo_vnode_id),
        signature_vnode_id: opt_vnode_id(&form.company_signature_vnode_id),
    }
}

/// HTTP handler: `post /work-orders/pdf/preview/work-order`.
pub async fn work_order_pdf_preview_post(
    Cap(state): Cap<WorkOrdersState>,
    Cap(fs): Cap<FilesystemState>,
    RequireAuth(ctx): RequireAuth,
    HtmlFormBody(form): HtmlFormBody<WorkOrdersPreferencesForm>,
) -> Markup {
    if !require_superuser(&ctx) {
        return render_preview_modal("", Some("Forbidden"));
    }
    cleanup_stale_previews(3600);
    let template = if form.draft_work_order_pdf_template.trim().is_empty() {
        None
    } else {
        Some(form.draft_work_order_pdf_template.as_str())
    };
    let presentation = company_presentation_from_form(&form);
    match pdf::render_work_order_pdf_preview(
        &state.db,
        Some(&fs),
        template,
        Some(presentation),
        &ctx.timezone,
    )
    .await
    {
        Ok(result) => {
            let token = preview_token();
            if let Err(msg) = store_preview_pdf(&token, &result.bytes) {
                return render_preview_modal("", Some(&msg));
            }
            let pdf_url = WorkOrdersPdfPreviewPdfRouteTag::new(token).url();
            render_preview_modal(&pdf_url, None)
        }
        Err(PdfError::Message(msg)) => render_preview_modal("", Some(&msg)),
        Err(PdfError::NotFound) => render_preview_modal("", Some("Not found")),
    }
}

/// HTTP handler: `post /work-orders/pdf/preview/issued-work-order`.
pub async fn issued_work_order_pdf_preview_post(
    Cap(state): Cap<WorkOrdersState>,
    Cap(fs): Cap<FilesystemState>,
    RequireAuth(ctx): RequireAuth,
    HtmlFormBody(form): HtmlFormBody<WorkOrdersPreferencesForm>,
) -> Markup {
    if !require_superuser(&ctx) {
        return render_preview_modal("", Some("Forbidden"));
    }
    cleanup_stale_previews(3600);
    let template = if form.work_order_pdf_template.trim().is_empty() {
        None
    } else {
        Some(form.work_order_pdf_template.as_str())
    };
    let presentation = company_presentation_from_form(&form);
    match pdf::render_issued_work_order_pdf_preview(
        &state.db,
        Some(&fs),
        template,
        Some(presentation),
        &ctx.timezone,
    )
    .await
    {
        Ok(result) => {
            let token = preview_token();
            if let Err(msg) = store_preview_pdf(&token, &result.bytes) {
                return render_preview_modal("", Some(&msg));
            }
            let pdf_url = WorkOrdersPdfPreviewPdfRouteTag::new(token).url();
            render_preview_modal(&pdf_url, None)
        }
        Err(PdfError::Message(msg)) => render_preview_modal("", Some(&msg)),
        Err(PdfError::NotFound) => render_preview_modal("", Some("Not found")),
    }
}

/// HTTP handler: `post /work-orders/pdf/preview/invoice`.
pub async fn invoice_pdf_preview_post(
    Cap(state): Cap<WorkOrdersState>,
    Cap(fs): Cap<FilesystemState>,
    RequireAuth(ctx): RequireAuth,
    HtmlFormBody(form): HtmlFormBody<WorkOrdersPreferencesForm>,
) -> Markup {
    if !require_superuser(&ctx) {
        return render_preview_modal("", Some("Forbidden"));
    }
    cleanup_stale_previews(3600);
    let template = if form.quotation_pdf_template.trim().is_empty() {
        None
    } else {
        Some(form.quotation_pdf_template.as_str())
    };
    let presentation = company_presentation_from_form(&form);
    match pdf::render_quotation_pdf_preview(&state.db, Some(&fs), template, Some(presentation))
        .await
    {
        Ok(result) => {
            let token = preview_token();
            if let Err(msg) = store_preview_pdf(&token, &result.bytes) {
                return render_preview_modal("", Some(&msg));
            }
            let pdf_url = WorkOrdersPdfPreviewPdfRouteTag::new(token).url();
            render_preview_modal(&pdf_url, None)
        }
        Err(PdfError::Message(msg)) => render_preview_modal("", Some(&msg)),
        Err(PdfError::NotFound) => render_preview_modal("", Some("Not found")),
    }
}

/// HTTP handler: `get /work-orders/pdf/preview/{token}`.
pub async fn preview_pdf_get(RequireAuth(ctx): RequireAuth, Path(token): Path<String>) -> Response {
    if !require_superuser(&ctx) {
        return StatusCode::FORBIDDEN.into_response();
    }
    if !is_valid_preview_token(&token) {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let path = preview_pdf_path(&token);
    if !path.starts_with(preview_cache_dir()) {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(_) => return StatusCode::NOT_FOUND.into_response(),
    };
    remove_preview_pdf(&token);
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/pdf".to_string()),
            (
                header::CONTENT_DISPOSITION,
                "inline; filename=\"work-orders-preview.pdf\"".to_string(),
            ),
        ],
        Body::from(bytes),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_token_validation_rejects_path_traversal() {
        assert!(!is_valid_preview_token("../etc/passwd"));
        assert!(!is_valid_preview_token(""));
        assert!(is_valid_preview_token("12345-67890"));
    }

    #[test]
    fn preview_pdf_path_stays_in_cache_dir() {
        let path = preview_pdf_path("abc-123");
        assert!(path.starts_with(preview_cache_dir()));
    }

    #[test]
    fn invoice_material_lines_parse_widget_snake_case_json() {
        let json = r#"[{"id":null,"component_id":1,"variables":"{\"length\":1000}","quantity":"10","unit_weight":"1.4","material_rate":"250","final_cost":"3500","extra_data":"{}"}]"#;
        let lines: Vec<crate::work_orders::forms::InvoiceMaterialLineInput> =
            serde_json::from_str(json).expect("widget snake_case json must parse");
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].component_id, 1);
    }

    #[test]
    fn invoice_machine_lines_parse_widget_snake_case_json() {
        let json = r#"[{"id":null,"machine_id":1,"variables":{"duration":"2h 30m"}}]"#;
        let lines: Vec<crate::work_orders::forms::InvoiceMachineLineInput> =
            serde_json::from_str(json).expect("widget snake_case json must parse");
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].machine_id, 1);
    }

    #[test]
    fn invoice_line_inputs_still_accept_pascal_case_json() {
        let material: Vec<crate::work_orders::forms::InvoiceMaterialLineInput> =
            serde_json::from_str(r#"[{"ComponentId":2,"Variables":{"qty":1}}]"#)
                .expect("pascal case json must parse");
        assert_eq!(material[0].component_id, 2);

        let machine: Vec<crate::work_orders::forms::InvoiceMachineLineInput> =
            serde_json::from_str(r#"[{"MachineId":3,"Variables":{"duration":"1h"}}]"#)
                .expect("pascal case json must parse");
        assert_eq!(machine[0].machine_id, 3);
    }

    #[test]
    fn validate_machine_lines_json_accepts_variables() {
        assert!(validate_machine_lines_json(None).is_ok());
        assert!(validate_machine_lines_json(Some("[]")).is_ok());

        let valid = r#"[{"id":null,"machine_id":2,"variables":{"duration":"2h"}}]"#;
        assert!(validate_machine_lines_json(Some(valid)).is_ok());

        let invalid = r#"[{"machine_id":"nope"}]"#;
        assert!(validate_machine_lines_json(Some(invalid)).is_err());
    }
}
