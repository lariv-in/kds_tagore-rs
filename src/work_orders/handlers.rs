use std::collections::HashMap;
use std::path::PathBuf;
use axum::{
    Form, Json,
    body::Body,
    extract::{Path, Query},
    http::{StatusCode, Uri, header},
    response::{IntoResponse, Redirect, Response},
};
use chrono::Utc;
use lariv_rs::{
    components::{SharedChromeFolder, SlotCtx, SwapKey, modal_keyed},
    html_form::HtmlFormBody,
    http::Cap,
    picker::respond_picker_select,
    plugins::{
        finance_common::require_superuser,
        users::middleware::{OptionalAuth, RequireAuth, RequireStaff},
    },
    template::RenderAppPane,
    web::{
        Htmx, ModalFormQuery, html_built_page_or_app_layout, html_built_page_with_slots,
        respond_create_modal_done, respond_edit_modal_done,
    },
};
use maud::{Markup, html};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter, QueryOrder,
};
use serde::{Deserialize, Serialize};

use std::str::FromStr;

use super::{
    entities::{
        WorkOrdersPreferences, component, draft_work_order_machine_line,
        draft_work_order_material_line, machine, material, material_rate, proforma_invoice,
        proforma_invoice_machine_line, proforma_invoice_material_line, shape, work_order,
        work_order_line,
    },
    forms::WorkOrdersPreferencesForm,
    keys::*,
    pdf::{self, PdfError, PdfResult},
    preferences::{empty_preferences, load_preferences, save_preferences},
    routes::*,
    state::WorkOrdersState,
    templates::*,
};

use crate::machinery_schedule::logic::{format_job_duration, parse_job_duration};

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
    let orders = work_order::Entity::find()
        .order_by_desc(work_order::Column::Id)
        .all(&state.db)
        .await
        .unwrap_or_default();

    let all_lines = work_order_line::Entity::find()
        .all(&state.db)
        .await
        .unwrap_or_default();

    let all_machine_lines = draft_work_order_machine_line::Entity::find()
        .all(&state.db)
        .await
        .unwrap_or_default();

    let mut lines_by_order: HashMap<i64, Vec<work_order_line::Model>> = HashMap::new();
    for line in all_lines {
        lines_by_order.entry(line.draft_work_order_id).or_default().push(line);
    }

    let mut machine_lines_by_order: HashMap<i64, Vec<draft_work_order_machine_line::Model>> = HashMap::new();
    for line in all_machine_lines {
        machine_lines_by_order.entry(line.draft_work_order_id).or_default().push(line);
    }

    let orders_with_stats: Vec<(work_order::Model, usize, Decimal)> = orders
        .into_iter()
        .map(|o| {
            let lines = lines_by_order.get(&o.id).map(|v| v.as_slice()).unwrap_or(&[]);
            let machine_lines = machine_lines_by_order.get(&o.id).map(|v| v.as_slice()).unwrap_or(&[]);
            let total = o.total_amount_with_machine_lines(lines, machine_lines);
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
    let Some(order) = work_order::Entity::find_by_id(id).one(&state.db).await.unwrap_or(None) else {
        return Redirect::to(&WorkOrdersDefaultRouteTag.url()).into_response();
    };

    let lines = work_order_line::Entity::find()
        .filter(work_order_line::Column::DraftWorkOrderId.eq(id))
        .order_by_asc(work_order_line::Column::Id)
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
    let machine_map: HashMap<i64, String> = machines.into_iter().map(|m| (m.id, m.name)).collect();

    let comp_ids: Vec<i64> = lines.iter().map(|l| l.component_id).collect();
    let comps = component::Entity::find()
        .filter(component::Column::Id.is_in(comp_ids))
        .all(&state.db)
        .await
        .unwrap_or_default();
    let comp_map: HashMap<i64, String> = comps.into_iter().map(|c| (c.id, c.name)).collect();

    let customer = lariv_rs::plugins::customer::entities::customer::Entity::find_by_id(order.customer_id)
        .one(&state.db)
        .await
        .ok()
        .flatten();

    let total_amount = order.total_amount_with_machine_lines(&lines, &machine_lines);

    let lines_with_comp: Vec<(work_order_line::Model, String)> = lines
        .into_iter()
        .map(|l| {
            let c_name = comp_map.get(&l.component_id).cloned().unwrap_or_else(|| format!("Component #{}", l.component_id));
            (l, c_name)
        })
        .collect();

    let machine_lines_with_name: Vec<(draft_work_order_machine_line::Model, String)> = machine_lines
        .into_iter()
        .map(|l| {
            let m_name = machine_map.get(&l.machine_id).cloned().unwrap_or_else(|| format!("Machine #{}", l.machine_id));
            (l, m_name)
        })
        .collect();

    let page = WorkOrderDetailPage {
        order,
        lines: lines_with_comp,
        machine_lines: machine_lines_with_name,
        customer_name: customer.map(|c| c.name),
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
    let mut query = work_order::Entity::find();
    if let Some(n) = q.name.as_deref().filter(|s| !s.trim().is_empty()) {
        query = query.filter(work_order::Column::OrderNumber.contains(n));
    }
    let orders = query
        .order_by_desc(work_order::Column::Id)
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

pub async fn fetch_components_meta(db: &sea_orm::DatabaseConnection) -> Vec<super::forms::ComponentMeta> {
    let comps = component::Entity::find().order_by_asc(component::Column::Name).all(db).await.unwrap_or_default();
    let shapes = shape::Entity::find().all(db).await.unwrap_or_default();
    let materials = material::Entity::find().all(db).await.unwrap_or_default();
    let rates = material_rate::Entity::find().order_by_desc(material_rate::Column::Datetime).all(db).await.unwrap_or_default();

    let mut latest_rate_by_mat: HashMap<i64, f64> = HashMap::new();
    for r in rates {
        latest_rate_by_mat.entry(r.material_id).or_insert_with(|| {
            r.rate()
        });
    }

    let shape_map: HashMap<i64, shape::Model> = shapes.into_iter().map(|s| (s.id, s)).collect();
    let mat_map: HashMap<i64, material::Model> = materials.into_iter().map(|m| (m.id, m)).collect();

    comps.into_iter().map(|c| {
        let (s_name, s_kind, var_names) = shape_map.get(&c.shape_id).map(|s| {
            (s.name.clone(), s.standard_kind().map(|k| format!("{:?}", k)), s.variable_names_vec())
        }).unwrap_or_default();

        let (m_name, density, m_rate) = mat_map.get(&c.material_id).map(|m| {
            let d = m.density;
            let r = latest_rate_by_mat.get(&m.id).copied().unwrap_or_default();
            (m.name.clone(), d, r)
        }).unwrap_or_default();

        let fixed = c.fixed_variables_map();
        let free: Vec<String> = var_names.iter().filter(|v| !fixed.contains_key(*v)).cloned().collect();

        super::forms::ComponentMeta {
            id: c.id,
            name: c.name,
            shape_id: c.shape_id,
            shape_name: s_name,
            shape_kind: s_kind,
            material_id: c.material_id,
            material_name: m_name,
            density,
            material_rate: m_rate,
            variable_names: var_names,
            fixed_variables: fixed,
            free_variables: free,
        }
    }).collect()
}

fn json_positive_f64(v: Option<&serde_json::Value>) -> Option<f64> {
    let n = match v? {
        serde_json::Value::Number(num) => num.as_f64()?,
        serde_json::Value::String(s) => s.trim().parse().ok()?,
        _ => return None,
    };
    (n > 0.0).then_some(n)
}

fn json_nonneg_f64(v: Option<&serde_json::Value>) -> Option<f64> {
    let n = match v? {
        serde_json::Value::Number(num) => num.as_f64()?,
        serde_json::Value::String(s) => s.trim().parse().ok()?,
        _ => return None,
    };
    n.is_finite().then_some(n).filter(|x| *x >= 0.0)
}

async fn resolve_and_compute_line_data(
    db: &sea_orm::DatabaseConnection,
    component_id: i64,
    variables_val: Option<&serde_json::Value>,
    target_weight: Option<f64>,
    target_cost: Option<f64>,
    quantity_val: &serde_json::Value,
    extra_data_val: Option<&serde_json::Value>,
    submitted_final_cost: Option<f64>,
    submitted_material_rate: Option<f64>,
) -> Result<(sea_orm::prelude::Json, Decimal, Decimal, Decimal, Decimal, sea_orm::prelude::Json), String> {

    let comp = component::Entity::find_by_id(component_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Component #{} not found", component_id))?;

    let shape = shape::Entity::find_by_id(comp.shape_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Shape #{} not found", comp.shape_id))?;

    let mat = material::Entity::find_by_id(comp.material_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Material #{} not found", comp.material_id))?;

    let latest_rate_model = material_rate::Entity::find()
        .filter(material_rate::Column::MaterialId.eq(comp.material_id))
        .order_by_desc(material_rate::Column::Datetime)
        .one(db)
        .await
        .ok()
        .flatten();

    let mut material_rate = latest_rate_model.as_ref().map(|r| r.rate_decimal).unwrap_or(Decimal::ZERO);
    let mut material_rate_f64 = latest_rate_model.as_ref().map(|r| r.rate()).unwrap_or(0.0);
    if let Some(r) = submitted_material_rate.filter(|x| *x >= 0.0) {
        material_rate = Decimal::from_f64_retain(r).unwrap_or(material_rate);
        material_rate_f64 = r;
    }

    let free_vars = comp.free_variable_names(&shape);
    let mut vars: HashMap<String, f64> = HashMap::new();

    if let Some(v) = variables_val {
        match v {
            serde_json::Value::Object(map) => {
                for (k, val) in map {
                    if let Some(n) = val.as_f64() {
                        vars.insert(k.clone(), n);
                    } else if let Some(s) = val.as_str() {
                        if let Ok(n) = s.trim().parse::<f64>() {
                            vars.insert(k.clone(), n);
                        }
                    }
                }
            }
            serde_json::Value::String(s) => {
                let trimmed = s.trim();
                if !trimmed.is_empty() {
                    if let Ok(map) = serde_json::from_str::<HashMap<String, serde_json::Value>>(trimmed) {
                        for (k, val) in map {
                            if let Some(n) = val.as_f64() {
                                vars.insert(k, n);
                            } else if let Some(sv) = val.as_str() {
                                if let Ok(n) = sv.trim().parse::<f64>() {
                                    vars.insert(k, n);
                                }
                            }
                        }
                    } else if let Ok(val) = trimmed.parse::<f64>() {
                        if free_vars.len() == 1 {
                            vars.insert(free_vars[0].clone(), val);
                        }
                    }
                }
            }
            serde_json::Value::Number(num) => {
                if let Some(val) = num.as_f64() {
                    if free_vars.len() == 1 {
                        vars.insert(free_vars[0].clone(), val);
                    }
                }
            }
            _ => {}
        }
    }

    let quantity = match quantity_val {
        serde_json::Value::Number(n) => Decimal::from_str(&n.to_string()).unwrap_or(Decimal::ONE),
        serde_json::Value::String(s) => Decimal::from_str(s.trim()).unwrap_or(Decimal::ONE),
        _ => Decimal::ONE,
    };

    let dims_complete = |vars: &HashMap<String, f64>| {
        free_vars.is_empty()
            || free_vars
                .iter()
                .all(|fv| vars.get(fv).copied().filter(|n| *n > 0.0).is_some())
    };

    if !dims_complete(&vars) {
        if let Some(w) = target_weight.filter(|x| *x > 0.0) {
            if free_vars.len() == 1 {
                if let Ok((_, solved_dim)) = comp.solve_final_variable_from_weight(&shape, &mat, w) {
                    vars.insert(free_vars[0].clone(), solved_dim);
                }
            }
        } else {
            let line_cost = submitted_final_cost.or(target_cost).filter(|x| *x > 0.0);
            if let Some(c) = line_cost {
                if free_vars.len() == 1 && material_rate_f64 > 0.0 {
                    let qty_f = quantity.to_string().parse::<f64>().unwrap_or(1.0);
                    let unit_cost = if qty_f > 0.0 { c / qty_f } else { c };
                    if let Ok((_, solved_dim)) =
                        comp.solve_final_variable_from_cost(&shape, &mat, material_rate_f64, unit_cost)
                    {
                        vars.insert(free_vars[0].clone(), solved_dim);
                    }
                }
            }
        }
    }

    let mut full_vars = comp.fixed_variables_map();
    for (k, v) in &vars {
        full_vars.insert(k.clone(), *v);
    }

    let geom_weight_f64 = component::Model::get_weight_from_models(&shape, &mat, full_vars);
    let mut unit_weight =
        Decimal::from_f64_retain(geom_weight_f64).unwrap_or(Decimal::ZERO).round_dp(4);

    let has_dims = dims_complete(&vars);
    let has_weight = target_weight.filter(|x| *x > 0.0);
    if !has_dims {
        if let Some(w) = has_weight {
            unit_weight = Decimal::from_f64_retain(w).unwrap_or(unit_weight).round_dp(4);
        }
    }

    let computed_cost =
        draft_work_order_material_line::Model::calculate_final_cost(quantity, material_rate, unit_weight);
    let final_cost = if has_dims || has_weight.is_some() {
        computed_cost
    } else if let Some(c) = submitted_final_cost.or(target_cost).filter(|x| *x > 0.0) {
        Decimal::from_f64_retain(c).unwrap_or(computed_cost).round_dp(2)
    } else {
        computed_cost
    };

    let extra_data = match extra_data_val {
        Some(serde_json::Value::Object(_)) | Some(serde_json::Value::Array(_)) => {
            extra_data_val.cloned().unwrap()
        }
        Some(serde_json::Value::String(s)) => {
            let trimmed = s.trim();
            if !trimmed.is_empty() {
                serde_json::from_str::<serde_json::Value>(trimmed).unwrap_or_else(|_| serde_json::json!({}))
            } else {
                serde_json::json!({})
            }
        }
        _ => serde_json::json!({}),
    };

    let vars_json = serde_json::to_value(&vars).unwrap_or_else(|_| serde_json::json!({}));
    Ok((vars_json, quantity, unit_weight, material_rate, final_cost, extra_data))
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
        items_json: "[]".into(),
        components_json,
        machine_lines_json: "[]".into(),
        machines_json,
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
                trimmed.parse::<i64>().map(Some).map_err(serde::de::Error::custom)
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
    #[serde(alias = "CustomerID", alias = "customer_id", default, deserialize_with = "i64_from_str_or_zero")]
    pub customer_id: i64,
    #[serde(alias = "items", default)]
    pub items: Option<String>,
    #[serde(alias = "machine_lines", default)]
    pub machine_lines: Option<String>,
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
            items_json: form.items.unwrap_or_default(),
            components_json,
            machine_lines_json: form.machine_lines.unwrap_or_default(),
            machines_json,
            error: "Please select a customer.".into(),
        };
        return html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response();
    }

    let model = work_order::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        order_number: Set(form.order_number.trim().to_string()),
        customer_id: Set(form.customer_id),
    };

    match model.insert(&state.db).await {
        Ok(saved) => {
            if let Some(items_str) = form.items.as_deref().filter(|s| !s.trim().is_empty()) {
                if let Ok(items) = serde_json::from_str::<Vec<super::forms::DraftWorkOrderLineInput>>(items_str) {
                    for item in items {
                        if item.component_id <= 0 {
                            continue;
                        }
                        if let Ok((vars_json, qty, unit_weight, material_rate, final_cost, extra_data)) =
                            resolve_and_compute_line_data(
                                &state.db,
                                item.component_id,
                                item.variables.as_ref(),
                                item.target_weight,
                                item.target_cost,
                                &item.quantity,
                                item.extra_data.as_ref(),
                                json_positive_f64(item.final_cost.as_ref()),
                                json_nonneg_f64(item.material_rate.as_ref()),
                            ).await
                        {
                            let line_am = draft_work_order_material_line::ActiveModel {
                                id: Default::default(),
                                created_at: Set(Some(now)),
                                updated_at: Set(Some(now)),
                                draft_work_order_id: Set(saved.id),
                                component_id: Set(item.component_id),
                                variables: Set(vars_json),
                                quantity: Set(qty),
                                unit_weight: Set(unit_weight),
                                material_rate: Set(material_rate),
                                final_cost: Set(final_cost),
                                extra_data: Set(extra_data),
                            };
                            let _ = line_am.insert(&state.db).await;
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
            let customer_name = lariv_rs::plugins::customer::entities::customer::Entity::find_by_id(form.customer_id)
                .one(&state.db)
                .await
                .ok()
                .flatten()
                .map(|c| c.name)
                .unwrap_or_default();
            let components = fetch_components_meta(&state.db).await;
            let components_json = serde_json::to_string(&components).unwrap_or_else(|_| "[]".into());
            let machines_json = fetch_machines_json(&state.db).await;
            let page = WorkOrderCreateModalPage {
                form_name: q.form_name(),
                order_number: form.order_number,
                customer_id: Some(form.customer_id),
                customer_name,
                items_json: form.items.unwrap_or_default(),
                components_json,
                machine_lines_json: form.machine_lines.unwrap_or_default(),
                machines_json,
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
    #[serde(alias = "CustomerID", alias = "customer_id", default, deserialize_with = "i64_from_str_or_zero")]
    pub customer_id: i64,
    #[serde(alias = "items", default)]
    pub items: Option<String>,
    #[serde(alias = "machine_lines", default)]
    pub machine_lines: Option<String>,
}

pub async fn work_order_edit_get(
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    Query(q): Query<ModalFormQuery>,
    Path(id): Path<i64>,
) -> Response {
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    let order = match work_order::Entity::find_by_id(id).one(&state.db).await {
        Ok(Some(o)) => o,
        _ => return Redirect::to(&WorkOrdersDefaultRouteTag.url()).into_response(),
    };
    let customer_name = lariv_rs::plugins::customer::entities::customer::Entity::find_by_id(order.customer_id)
        .one(&state.db)
        .await
        .ok()
        .flatten()
        .map(|c| c.name)
        .unwrap_or_default();

    let lines = work_order_line::Entity::find()
        .filter(work_order_line::Column::DraftWorkOrderId.eq(id))
        .order_by_asc(work_order_line::Column::Id)
        .all(&state.db)
        .await
        .unwrap_or_default();
    let items_json = serde_json::to_string(&lines).unwrap_or_else(|_| "[]".into());
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
        items_json,
        components_json,
        machine_lines_json,
        machines_json,
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
    eprintln!("[DEBUG edit_post id={id}] order_number={:?} customer_id={:?} items={:?} machine_lines={:?}", form.order_number, form.customer_id, form.items, form.machine_lines);
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    let existing = match work_order::Entity::find_by_id(id).one(&state.db).await {
        Ok(Some(o)) => o,
        _ => return Redirect::to(&WorkOrdersDefaultRouteTag.url()).into_response(),
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
            items_json: form.items.unwrap_or_default(),
            components_json,
            machine_lines_json: form.machine_lines.unwrap_or_default(),
            machines_json,
            error: "Please select a customer.".into(),
        };
        return html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response();
    }

    let now = Utc::now();
    let mut am: work_order::ActiveModel = existing.into();
    am.updated_at = Set(Some(now));
    am.order_number = Set(form.order_number.trim().to_string());
    am.customer_id = Set(form.customer_id);

    match am.update(&state.db).await {
        Ok(_) => {
            // Synchronize items if submitted
            if let Some(items_str) = form.items.as_deref().filter(|s| !s.trim().is_empty()) {
                if let Ok(items) = serde_json::from_str::<Vec<super::forms::DraftWorkOrderLineInput>>(items_str) {
                    let _ = work_order_line::Entity::delete_many()
                        .filter(work_order_line::Column::DraftWorkOrderId.eq(id))
                        .exec(&state.db)
                        .await;

                    for item in items {
                        if item.component_id <= 0 {
                            continue;
                        }
                        if let Ok((vars_json, qty, unit_weight, material_rate, final_cost, extra_data)) =
                            resolve_and_compute_line_data(
                                &state.db,
                                item.component_id,
                                item.variables.as_ref(),
                                item.target_weight,
                                item.target_cost,
                                &item.quantity,
                                item.extra_data.as_ref(),
                                json_positive_f64(item.final_cost.as_ref()),
                                json_nonneg_f64(item.material_rate.as_ref()),
                            ).await
                        {
                            let line_am = work_order_line::ActiveModel {
                                id: Default::default(),
                                created_at: Set(Some(now)),
                                updated_at: Set(Some(now)),
                                draft_work_order_id: Set(id),
                                component_id: Set(item.component_id),
                                variables: Set(vars_json),
                                quantity: Set(qty),
                                unit_weight: Set(unit_weight),
                                material_rate: Set(material_rate),
                                final_cost: Set(final_cost),
                                extra_data: Set(extra_data),
                            };
                            let _ = line_am.insert(&state.db).await;
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
            let customer_name = lariv_rs::plugins::customer::entities::customer::Entity::find_by_id(form.customer_id)
                .one(&state.db)
                .await
                .ok()
                .flatten()
                .map(|c| c.name)
                .unwrap_or_default();
            let components = fetch_components_meta(&state.db).await;
            let components_json = serde_json::to_string(&components).unwrap_or_else(|_| "[]".into());
            let machines_json = fetch_machines_json(&state.db).await;
            let page = WorkOrderEditModalPage {
                id,
                form_name: q.form_name(),
                order_number: form.order_number,
                customer_id: form.customer_id,
                customer_name,
                items_json: form.items.unwrap_or_default(),
                components_json,
                machine_lines_json: form.machine_lines.unwrap_or_default(),
                machines_json,
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
    let _ = work_order::Entity::delete_by_id(id).exec(&state.db).await;
    htmx.redirect(&WorkOrdersDefaultRouteTag.url())
}

// ==========================================
// 1b. DRAFT WORK ORDER LINES
// ==========================================

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct WorkOrderLineEditForm {
    #[serde(alias = "DraftWorkOrderID", alias = "draft_work_order_id", default, deserialize_with = "opt_i64_from_str")]
    pub draft_work_order_id: Option<i64>,
    #[serde(alias = "ComponentID", alias = "component_id", default, deserialize_with = "i64_from_str_or_zero")]
    pub component_id: i64,
    #[serde(alias = "variables", default)]
    pub variables: String,
    #[serde(alias = "quantity", default)]
    pub quantity: String,
    #[serde(alias = "extra_data", default)]
    pub extra_data: Option<String>,
}

pub async fn work_order_line_edit_get(
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    Query(q): Query<ModalFormQuery>,
    Path(id): Path<i64>,
) -> Response {
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    let line = match draft_work_order_material_line::Entity::find_by_id(id).one(&state.db).await {
        Ok(Some(l)) => l,
        _ => return Redirect::to(&WorkOrdersDefaultRouteTag.url()).into_response(),
    };
    let order = work_order::Entity::find_by_id(line.draft_work_order_id).one(&state.db).await.ok().flatten();
    let label = order.map(|o| format!("{} (#{})", o.order_number, o.id)).unwrap_or_else(|| format!("#{}", line.draft_work_order_id));

    let comp = component::Entity::find_by_id(line.component_id).one(&state.db).await.ok().flatten();
    let comp_label = comp.map(|c| c.name).unwrap_or_default();

    let extra_data = line.extra_data_str();
    let page = WorkOrderLineEditModalPage {
        id,
        draft_work_order_id: line.draft_work_order_id,
        draft_work_order_label: label,
        form_name: q.form_name(),
        component_id: line.component_id,
        component_label: comp_label,
        variables: serde_json::to_string(&line.variables).unwrap_or_else(|_| "{}".into()),
        quantity: line.quantity.to_string(),
        extra_data,
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
    let existing = match draft_work_order_material_line::Entity::find_by_id(id).one(&state.db).await {
        Ok(Some(l)) => l,
        _ => return Redirect::to(&WorkOrdersDefaultRouteTag.url()).into_response(),
    };
    let target_order_id = form.draft_work_order_id.unwrap_or(existing.draft_work_order_id);

    if form.component_id <= 0 {
        let order = work_order::Entity::find_by_id(target_order_id).one(&state.db).await.ok().flatten();
        let label = order.map(|o| format!("{} (#{})", o.order_number, o.id)).unwrap_or_else(|| format!("#{}", target_order_id));
        let page = WorkOrderLineEditModalPage {
            id,
            draft_work_order_id: target_order_id,
            draft_work_order_label: label,
            form_name: q.form_name(),
            component_id: form.component_id,
            component_label: String::new(),
            variables: form.variables,
            quantity: form.quantity,
            extra_data: form.extra_data.unwrap_or_default(),
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

    if form.quantity.trim().is_empty() {
        let order = work_order::Entity::find_by_id(target_order_id).one(&state.db).await.ok().flatten();
        let label = order.map(|o| format!("{} (#{})", o.order_number, o.id)).unwrap_or_else(|| format!("#{}", target_order_id));
        let page = WorkOrderLineEditModalPage {
            id,
            draft_work_order_id: target_order_id,
            draft_work_order_label: label,
            form_name: q.form_name(),
            component_id: form.component_id,
            component_label: comp_label,
            variables: form.variables,
            quantity: form.quantity,
            extra_data: form.extra_data.unwrap_or_default(),
            error: "Quantity is required.".into(),
        };
        return html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response();
    }

    let vars_val = serde_json::Value::String(form.variables.clone());
    let qty_val = serde_json::Value::String(form.quantity.clone());
    let extra_val = form.extra_data.clone().map(serde_json::Value::String);

    match resolve_and_compute_line_data(
        &state.db,
        form.component_id,
        Some(&vars_val),
        None,
        None,
        &qty_val,
        extra_val.as_ref(),
        None,
        None,
    ).await {
        Ok((vars_json, quantity, unit_weight, material_rate, final_cost, extra_data)) => {
            let now = Utc::now();
            let mut am: draft_work_order_material_line::ActiveModel = existing.clone().into();
            am.updated_at = Set(Some(now));
            am.draft_work_order_id = Set(target_order_id);
            am.component_id = Set(form.component_id);
            am.variables = Set(vars_json);
            am.quantity = Set(quantity);
            am.unit_weight = Set(unit_weight);
            am.material_rate = Set(material_rate);
            am.final_cost = Set(final_cost);
            am.extra_data = Set(extra_data);

            match am.update(&state.db).await {
                Ok(_) => respond_edit_modal_done::<WorkOrderLineEditModalKey>(
                    &htmx,
                    &WorkOrderDetailRouteTag::new(target_order_id).url(),
                ),
                Err(e) => {
                    let order = work_order::Entity::find_by_id(target_order_id).one(&state.db).await.ok().flatten();
                    let label = order.map(|o| format!("{} (#{})", o.order_number, o.id)).unwrap_or_else(|| format!("#{}", target_order_id));
                    let page = WorkOrderLineEditModalPage {
                        id,
                        draft_work_order_id: target_order_id,
                        draft_work_order_label: label,
                        form_name: q.form_name(),
                        component_id: form.component_id,
                        component_label: comp_label,
                        variables: form.variables,
                        quantity: form.quantity,
                        extra_data: form.extra_data.unwrap_or_default(),
                        error: e.to_string(),
                    };
                    html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response()
                }
            }
        }
        Err(err) => {
            let order = work_order::Entity::find_by_id(target_order_id).one(&state.db).await.ok().flatten();
            let label = order.map(|o| format!("{} (#{})", o.order_number, o.id)).unwrap_or_else(|| format!("#{}", target_order_id));
            let page = WorkOrderLineEditModalPage {
                id,
                draft_work_order_id: target_order_id,
                draft_work_order_label: label,
                form_name: q.form_name(),
                component_id: form.component_id,
                component_label: comp_label,
                variables: form.variables,
                quantity: form.quantity,
                extra_data: form.extra_data.unwrap_or_default(),
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
    let draft_work_order_id = if let Ok(Some(line)) = draft_work_order_material_line::Entity::find_by_id(id).one(&state.db).await {
        let wid = line.draft_work_order_id;
        let _ = draft_work_order_material_line::Entity::delete_by_id(id).exec(&state.db).await;
        wid
    } else {
        return htmx.redirect(&WorkOrdersDefaultRouteTag.url());
    };
    htmx.redirect(&WorkOrderDetailRouteTag::new(draft_work_order_id).url())
}

// ==========================================
// 1c. DRAFT WORK ORDER MACHINE LINES
// ==========================================

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct WorkOrderMachineLineFormData {
    #[serde(alias = "DraftWorkOrderID", alias = "draft_work_order_id", default, deserialize_with = "opt_i64_from_str")]
    pub draft_work_order_id: Option<i64>,
    #[serde(alias = "MachineID", alias = "machine_id", default, deserialize_with = "i64_from_str_or_zero")]
    pub machine_id: i64,
    #[serde(alias = "Rate", alias = "rate", default)]
    pub rate: String,
    #[serde(alias = "Duration", alias = "duration", default)]
    pub duration: String,
}

pub async fn machine_select(
    Cap(state): Cap<WorkOrdersState>,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<EntitySelectQuery>,
) -> maud::Markup {
    let mut query = machine::Entity::find();
    if let Some(n) = q.name.as_deref().filter(|s| !s.trim().is_empty()) {
        query = query.filter(machine::Column::Name.contains(n));
    }
    let machines = query
        .order_by_asc(machine::Column::Name)
        .all(&state.db)
        .await
        .unwrap_or_default();

    let items: Vec<(machine::Model, String)> = machines
        .into_iter()
        .map(|m| {
            let rate_str = format!("₹ {:.2}", m.rate_decimal);
            (m, rate_str)
        })
        .collect();

    let page = MachineSelectPage {
        machines: items,
        target_input: q.target_input.unwrap_or_else(|| "machine_id".into()),
        path_and_query: uri.to_string(),
    };
    respond_picker_select::<MachineSelectTableKey, MachineSelectModalKey, _>(&htmx, &page)
}

async fn machine_line_rate_default(
    db: &sea_orm::DatabaseConnection,
    machine_id: i64,
    submitted: &str,
) -> Decimal {
    let machine_rate = machine::Entity::find_by_id(machine_id)
        .one(db)
        .await
        .ok()
        .flatten()
        .map(|m| m.rate_decimal)
        .unwrap_or(Decimal::ZERO);
    let parsed = Decimal::from_str(submitted.trim()).ok().filter(|d| !d.is_zero());
    parsed.unwrap_or(machine_rate)
}

async fn machine_line_machine_label(
    db: &sea_orm::DatabaseConnection,
    machine_id: i64,
) -> String {
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
                "rate_decimal": m.rate_decimal.to_string(),
            })
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
    let rows: Vec<serde_json::Value> = machine_lines
        .into_iter()
        .map(|l| {
            serde_json::json!({
                "id": l.id,
                "db_id": l.id,
                "machine_id": l.machine_id,
                "rate": l.rate_decimal.to_string(),
                "duration": format_job_duration(l.time_used),
            })
        })
        .collect();
    serde_json::to_string(&rows).unwrap_or_else(|_| "[]".into())
}

async fn sync_machine_lines(
    db: &sea_orm::DatabaseConnection,
    order_id: i64,
    ml_str: Option<String>,
    now: chrono::DateTime<chrono::Utc>,
) {
    let Some(s) = ml_str.as_deref().filter(|s| !s.trim().is_empty()) else {
        eprintln!("[DEBUG sync_machine_lines] ml_str absent/empty");
        return;
    };
    eprintln!("[DEBUG sync_machine_lines] raw={s}");
    let Ok(lines) = serde_json::from_str::<Vec<super::forms::DraftWorkOrderMachineLineInput>>(s) else {
        eprintln!("[DEBUG sync_machine_lines] JSON parse failed");
        return;
    };
    eprintln!("[DEBUG sync_machine_lines] parsed {} lines", lines.len());
    let _ = draft_work_order_machine_line::Entity::delete_many()
        .filter(draft_work_order_machine_line::Column::DraftWorkOrderId.eq(order_id))
        .exec(db)
        .await;
    for line in lines {
        if line.machine_id <= 0 || line.duration.trim().is_empty() {
            continue;
        }
        let Ok(time_used) = parse_job_duration(line.duration.trim()) else {
            continue;
        };
        let rate_submitted = match line.rate.as_ref() {
            Some(serde_json::Value::String(s)) => s.clone(),
            Some(v) => v.to_string(),
            None => String::new(),
        };
        let rate = machine_line_rate_default(db, line.machine_id, &rate_submitted).await;
        let am = draft_work_order_machine_line::ActiveModel {
            id: Default::default(),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            draft_work_order_id: Set(order_id),
            machine_id: Set(line.machine_id),
            rate_decimal: Set(rate),
            time_used: Set(time_used),
        };
        let _ = am.insert(db).await;
    }
}

async fn fetch_materials_json(db: &sea_orm::DatabaseConnection) -> String {
    let materials = material::Entity::find()
        .order_by_asc(material::Column::Name)
        .all(db)
        .await
        .unwrap_or_default();
    let rates = material_rate::Entity::find()
        .order_by_desc(material_rate::Column::Datetime)
        .order_by_desc(material_rate::Column::Id)
        .all(db)
        .await
        .unwrap_or_default();
    let mut latest_rate_by_mat: HashMap<i64, Decimal> = HashMap::new();
    for r in rates {
        latest_rate_by_mat.entry(r.material_id).or_insert(r.rate_decimal);
    }
    let rows: Vec<serde_json::Value> = materials
        .into_iter()
        .map(|m| {
            serde_json::json!({
                "id": m.id,
                "name": m.name,
                "rate": latest_rate_by_mat.get(&m.id).map(|r| r.to_string()).unwrap_or_else(|| "0".into()),
            })
        })
        .collect();
    serde_json::to_string(&rows).unwrap_or_else(|_| "[]".into())
}

async fn invoice_material_lines_json(db: &sea_orm::DatabaseConnection, invoice_id: i64) -> String {
    let lines = proforma_invoice_material_line::Entity::find()
        .filter(proforma_invoice_material_line::Column::InvoiceId.eq(invoice_id))
        .order_by_asc(proforma_invoice_material_line::Column::Id)
        .all(db)
        .await
        .unwrap_or_default();
    let rows: Vec<serde_json::Value> = lines
        .into_iter()
        .map(|l| {
            serde_json::json!({
                "id": l.id,
                "db_id": l.id,
                "material_id": l.material_id,
                "name": l.name,
                "qty": l.qty_decimal.to_string(),
                "rate": l.rate_decimal.to_string(),
                "amount": l.line_total().to_string().parse::<f64>().unwrap_or(0.0),
            })
        })
        .collect();
    serde_json::to_string(&rows).unwrap_or_else(|_| "[]".into())
}

async fn invoice_machine_lines_json(db: &sea_orm::DatabaseConnection, invoice_id: i64) -> String {
    let lines = proforma_invoice_machine_line::Entity::find()
        .filter(proforma_invoice_machine_line::Column::InvoiceId.eq(invoice_id))
        .order_by_asc(proforma_invoice_machine_line::Column::Id)
        .all(db)
        .await
        .unwrap_or_default();
    let rows: Vec<serde_json::Value> = lines
        .into_iter()
        .map(|l| {
            serde_json::json!({
                "id": l.id,
                "db_id": l.id,
                "machine_id": l.machine_id,
                "name": l.name,
                "duration": format_job_duration(l.time_used),
                "rate": l.rate_decimal.to_string(),
                "amount": l.line_total().to_string().parse::<f64>().unwrap_or(0.0),
            })
        })
        .collect();
    serde_json::to_string(&rows).unwrap_or_else(|_| "[]".into())
}

async fn sync_invoice_lines(
    db: &sea_orm::DatabaseConnection,
    invoice_id: i64,
    material_lines_str: Option<String>,
    machine_lines_str: Option<String>,
    now: chrono::DateTime<chrono::Utc>,
) {
    if let Some(s) = material_lines_str.as_deref().filter(|s| !s.trim().is_empty()) {
        if let Ok(lines) = serde_json::from_str::<Vec<super::forms::InvoiceMaterialLineInput>>(s) {
            let _ = proforma_invoice_material_line::Entity::delete_many()
                .filter(proforma_invoice_material_line::Column::InvoiceId.eq(invoice_id))
                .exec(db)
                .await;
            for line in lines {
                if line.name.trim().is_empty() { continue; }
                let material_id = line.material_id.filter(|&id| id > 0);
                let mut qty: Decimal = line.qty.parse().unwrap_or(Decimal::ZERO);
                let rate: Decimal = line.rate.parse().unwrap_or(Decimal::ZERO);
                if qty <= Decimal::ZERO {
                    if let Some(amount) = line.amount.filter(|a| *a > 0.0) {
                        if rate > Decimal::ZERO {
                            let amt = Decimal::from_f64_retain(amount).unwrap_or(Decimal::ZERO);
                            qty = (amt / rate).round_dp(6);
                        }
                    }
                }
                let am = proforma_invoice_material_line::ActiveModel {
                    id: Default::default(),
                    created_at: Set(Some(now)),
                    updated_at: Set(Some(now)),
                    invoice_id: Set(invoice_id),
                    material_id: Set(material_id),
                    name: Set(line.name.clone()),
                    rate_decimal: Set(rate),
                    qty_decimal: Set(qty),
                };
                let _ = am.insert(db).await;
            }
        }
    }
    if let Some(s) = machine_lines_str.as_deref().filter(|s| !s.trim().is_empty()) {
        if let Ok(lines) = serde_json::from_str::<Vec<super::forms::InvoiceMachineLineInput>>(s) {
            let _ = proforma_invoice_machine_line::Entity::delete_many()
                .filter(proforma_invoice_machine_line::Column::InvoiceId.eq(invoice_id))
                .exec(db)
                .await;
            for line in lines {
                let duration_str = line.duration.trim();
                let rate: Decimal = line.rate.parse().unwrap_or(Decimal::ZERO);
                let time_used = if !duration_str.is_empty() {
                    match parse_job_duration(duration_str) {
                        Ok(d) => d,
                        Err(_) => continue,
                    }
                } else if let Some(amount) = line.amount.filter(|a| *a > 0.0) {
                    if rate <= Decimal::ZERO {
                        continue;
                    }
                    let amt = amount / rate.to_string().parse::<f64>().unwrap_or(0.0);
                    if amt <= 0.0 {
                        continue;
                    }
                    crate::machinery_schedule::duration::JobDuration(chrono::Duration::nanoseconds(
                        (amt * 3_600_000_000_000.0) as i64,
                    ))
                } else {
                    continue;
                };
                if line.name.trim().is_empty() {
                    continue;
                }
                let machine_id = line.machine_id.filter(|&id| id > 0);
                let am = proforma_invoice_machine_line::ActiveModel {
                    id: Default::default(),
                    created_at: Set(Some(now)),
                    updated_at: Set(Some(now)),
                    invoice_id: Set(invoice_id),
                    machine_id: Set(machine_id),
                    name: Set(line.name.clone()),
                    time_used: Set(time_used),
                    rate_decimal: Set(rate),
                };
                let _ = am.insert(db).await;
            }
        }
    }
}

pub async fn work_order_machine_line_edit_get(
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    Query(q): Query<ModalFormQuery>,
    Path(id): Path<i64>,
) -> Response {
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    let line = match draft_work_order_machine_line::Entity::find_by_id(id).one(&state.db).await {
        Ok(Some(l)) => l,
        _ => return Redirect::to(&WorkOrdersDefaultRouteTag.url()).into_response(),
    };
    let order = work_order::Entity::find_by_id(line.draft_work_order_id).one(&state.db).await.ok().flatten();
    let label = order.map(|o| format!("{} (#{})", o.order_number, o.id)).unwrap_or_else(|| format!("#{}", line.draft_work_order_id));

    let machine_label = machine_line_machine_label(&state.db, line.machine_id).await;

    let page = WorkOrderMachineLineEditModalPage {
        id,
        form_name: q.form_name(),
        draft_work_order_id: line.draft_work_order_id,
        draft_work_order_label: label,
        machine_id: line.machine_id,
        machine_label,
        rate: line.rate_decimal.to_string(),
        duration: format_job_duration(line.time_used),
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
    let existing = match draft_work_order_machine_line::Entity::find_by_id(id).one(&state.db).await {
        Ok(Some(l)) => l,
        _ => return Redirect::to(&WorkOrdersDefaultRouteTag.url()).into_response(),
    };
    let target_order_id = form.draft_work_order_id.unwrap_or(existing.draft_work_order_id);

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
            form.rate,
            form.duration,
        )
        .await;
    }

    let machine_label = machine_line_machine_label(&state.db, form.machine_id).await;

    let duration = match parse_job_duration(form.duration.trim()) {
        Ok(d) => d,
        Err(e) => {
            return late_render_edit_error(
                &state.db,
                &chrome,
                &slot_ctx,
                &q,
                id,
                target_order_id,
                format!("Invalid duration: {e}"),
                form.machine_id,
                machine_label,
                form.rate,
                form.duration,
            )
            .await;
        }
    };

    let rate = machine_line_rate_default(&state.db, form.machine_id, &form.rate).await;

    let now = Utc::now();
    let mut am: draft_work_order_machine_line::ActiveModel = existing.into();
    am.updated_at = Set(Some(now));
    am.draft_work_order_id = Set(target_order_id);
    am.machine_id = Set(form.machine_id);
    am.rate_decimal = Set(rate);
    am.time_used = Set(duration);

    match am.update(&state.db).await {
        Ok(_) => respond_edit_modal_done::<WorkOrderMachineLineEditModalKey>(
            &htmx,
            &WorkOrderDetailRouteTag::new(target_order_id).url(),
        ),
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
                form.rate,
                form.duration,
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
    rate: String,
    duration: String,
) -> Response {
    let order = work_order::Entity::find_by_id(target_order_id).one(db).await.ok().flatten();
    let label = order.map(|o| format!("{} (#{})", o.order_number, o.id)).unwrap_or_else(|| format!("#{}", target_order_id));
    let page = WorkOrderMachineLineEditModalPage {
        id,
        form_name: q.form_name(),
        draft_work_order_id: target_order_id,
        draft_work_order_label: label,
        machine_id,
        machine_label,
        rate,
        duration,
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
    let draft_work_order_id = if let Ok(Some(line)) = draft_work_order_machine_line::Entity::find_by_id(id).one(&state.db).await {
        let wid = line.draft_work_order_id;
        let _ = draft_work_order_machine_line::Entity::delete_by_id(id).exec(&state.db).await;
        wid
    } else {
        return htmx.redirect(&WorkOrdersDefaultRouteTag.url());
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

    let shapes = shape::Entity::find().all(&state.db).await.unwrap_or_default();
    let materials = material::Entity::find().all(&state.db).await.unwrap_or_default();
    let shape_map: HashMap<i64, String> = shapes.into_iter().map(|s| (s.id, s.name)).collect();
    let mat_map: HashMap<i64, String> = materials.iter().map(|m| (m.id, m.name.clone())).collect();
    let rates = material_rate::Entity::find()
        .order_by_desc(material_rate::Column::Datetime)
        .all(&state.db)
        .await
        .unwrap_or_default();
    let mut latest_rate_by_mat: HashMap<i64, f64> = HashMap::new();
    for r in rates {
        latest_rate_by_mat.entry(r.material_id).or_insert_with(|| r.rate());
    }

    let items = components
        .into_iter()
        .map(|c| {
            let s_name = shape_map.get(&c.shape_id).cloned().unwrap_or_default();
            let m_name = mat_map.get(&c.material_id).cloned().unwrap_or_default();
            let rate = latest_rate_by_mat.get(&c.material_id).copied().unwrap_or(0.0);
            (c, s_name, m_name, rate)
        })
        .collect();

    let page = ComponentSelectPage {
        components: items,
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

    let mut result = Vec::with_capacity(comps.len());
    for c in comps {
        let s = shape::Entity::find_by_id(c.shape_id).one(&state.db).await.unwrap_or(None);
        let m = material::Entity::find_by_id(c.material_id).one(&state.db).await.unwrap_or(None);
        result.push((c, s, m));
    }

    let page = ComponentListPage {
        components: result,
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
    let Some(comp) = component::Entity::find_by_id(id).one(&state.db).await.unwrap_or(None) else {
        return Redirect::to(&WorkOrdersComponentsRouteTag.url()).into_response();
    };

    let shape = shape::Entity::find_by_id(comp.shape_id).one(&state.db).await.unwrap_or(None);
    let material = material::Entity::find_by_id(comp.material_id).one(&state.db).await.unwrap_or(None);
    let latest_rate = material_rate::Entity::find()
        .filter(material_rate::Column::MaterialId.eq(comp.material_id))
        .order_by_desc(material_rate::Column::Datetime)
        .order_by_desc(material_rate::Column::Id)
        .one(&state.db)
        .await
        .unwrap_or(None);

    let page = ComponentDetailPage {
        component: comp,
        shape,
        material,
        latest_rate,
    };
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx).into_response()
}

pub use super::forms::{
    is_allowed_variable_name, ComponentCreateForm, ComponentEditForm, ComponentForm,
    ALLOWED_VARIABLE_NAMES,
};

pub async fn component_create_get(
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    Query(q): Query<ModalFormQuery>,
) -> maud::Markup {
    let _ = crate::work_orders::seed::ensure_standard_seeds(&state.db).await;

    let shapes = shape::Entity::find().all(&state.db).await.unwrap_or_default();
    let all_shapes: Vec<(i64, Vec<String>)> = shapes
        .iter()
        .map(|s| (s.id, s.variable_names_vec()))
        .collect();

    let page = ComponentCreateModalPage {
        form_name: q.form_name(),
        name: String::new(),
        shape_id: None,
        shape_name: String::new(),
        shape_variables: Vec::new(),
        all_shapes,
        material_id: None,
        material_name: String::new(),
        fixed_variables: "{}".to_string(),
        error: String::new(),
    };
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
    let raw_fixed: HashMap<String, f64> = form
        .fixed_variables
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();

    let shapes = shape::Entity::find().all(&state.db).await.unwrap_or_default();
    let all_shapes: Vec<(i64, Vec<String>)> = shapes
        .iter()
        .map(|s| (s.id, s.variable_names_vec()))
        .collect();
    let cur_shape = shapes.iter().find(|s| s.id == form.shape_id);
    let shape_name = cur_shape.map(|s| s.name.clone()).unwrap_or_default();
    let shape_variables = cur_shape.map(|s| s.variable_names_vec()).unwrap_or_default();

    if form.name.trim().is_empty() {
        let material_name = material::Entity::find_by_id(form.material_id)
            .one(&state.db)
            .await
            .ok()
            .flatten()
            .map(|m| m.name)
            .unwrap_or_default();
        let page = ComponentCreateModalPage {
            form_name: q.form_name(),
            name: form.name,
            shape_id: if form.shape_id > 0 { Some(form.shape_id) } else { None },
            shape_name,
            shape_variables,
            all_shapes,
            material_id: if form.material_id > 0 { Some(form.material_id) } else { None },
            material_name,
            fixed_variables: form.fixed_variables.unwrap_or_default(),
            error: "Component name is required.".into(),
        };
        return html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response();
    }

    if form.shape_id <= 0 {
        let material_name = material::Entity::find_by_id(form.material_id)
            .one(&state.db)
            .await
            .ok()
            .flatten()
            .map(|m| m.name)
            .unwrap_or_default();
        let page = ComponentCreateModalPage {
            form_name: q.form_name(),
            name: form.name,
            shape_id: None,
            shape_name,
            shape_variables,
            all_shapes,
            material_id: if form.material_id > 0 { Some(form.material_id) } else { None },
            material_name,
            fixed_variables: form.fixed_variables.unwrap_or_default(),
            error: "Please select a shape.".into(),
        };
        return html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response();
    }

    if form.material_id <= 0 {
        let page = ComponentCreateModalPage {
            form_name: q.form_name(),
            name: form.name,
            shape_id: if form.shape_id > 0 { Some(form.shape_id) } else { None },
            shape_name,
            shape_variables,
            all_shapes,
            material_id: None,
            material_name: String::new(),
            fixed_variables: form.fixed_variables.unwrap_or_default(),
            error: "Please select a material.".into(),
        };
        return html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response();
    }

    for key in raw_fixed.keys() {
        let is_valid = if !shape_variables.is_empty() {
            shape_variables.contains(key)
        } else {
            is_allowed_variable_name(key)
        };
        if !is_valid {
            let material_name = material::Entity::find_by_id(form.material_id)
                .one(&state.db)
                .await
                .ok()
                .flatten()
                .map(|m| m.name)
                .unwrap_or_default();
            let page = ComponentCreateModalPage {
                form_name: q.form_name(),
                name: form.name,
                shape_id: Some(form.shape_id),
                shape_name: shape_name.clone(),
                shape_variables: shape_variables.clone(),
                all_shapes,
                material_id: Some(form.material_id),
                material_name,
                fixed_variables: form.fixed_variables.unwrap_or_default(),
                error: format!(
                    "Variable '{}' is not valid for shape '{}'. Allowed variables: {}",
                    key,
                    shape_name,
                    if !shape_variables.is_empty() {
                        shape_variables.join(", ")
                    } else {
                        ALLOWED_VARIABLE_NAMES.join(", ")
                    }
                ),
            };
            return html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response();
        }
    }
    let fixed_vars = raw_fixed;

    let now = Utc::now();
    let model = component::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        name: Set(form.name.trim().to_string()),
        shape_id: Set(form.shape_id),
        material_id: Set(form.material_id),
        fixed_variables: Set(serde_json::to_value(&fixed_vars).unwrap_or_default()),
    };

    match model.insert(&state.db).await {
        Ok(saved) => respond_create_modal_done::<ComponentCreateModalKey>(
            &htmx,
            &q.refresh_table(),
            &ComponentDetailRouteTag::new(saved.id).url(),
        ),
        Err(e) => {
            let material_name = material::Entity::find_by_id(form.material_id)
                .one(&state.db)
                .await
                .ok()
                .flatten()
                .map(|m| m.name)
                .unwrap_or_default();
            let page = ComponentCreateModalPage {
                form_name: q.form_name(),
                name: form.name,
                shape_id: Some(form.shape_id),
                shape_name,
                shape_variables,
                all_shapes,
                material_id: Some(form.material_id),
                material_name,
                fixed_variables: form.fixed_variables.unwrap_or_default(),
                error: e.to_string(),
            };
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

    let shapes = shape::Entity::find().all(&state.db).await.unwrap_or_default();
    let all_shapes: Vec<(i64, Vec<String>)> = shapes
        .iter()
        .map(|s| (s.id, s.variable_names_vec()))
        .collect();
    let cur_shape = shapes.iter().find(|s| s.id == comp.shape_id);
    let shape_name = cur_shape.map(|s| s.name.clone()).unwrap_or_default();
    let shape_variables = cur_shape.map(|s| s.variable_names_vec()).unwrap_or_default();

    let material_name = material::Entity::find_by_id(comp.material_id)
        .one(&state.db)
        .await
        .ok()
        .flatten()
        .map(|m| m.name)
        .unwrap_or_default();
    let fixed_vars_str = serde_json::to_string(&comp.fixed_variables).unwrap_or_else(|_| "{}".into());

    let page = ComponentEditModalPage {
        id,
        form_name: q.form_name(),
        name: comp.name,
        shape_id: comp.shape_id,
        shape_name,
        shape_variables,
        all_shapes,
        material_id: comp.material_id,
        material_name,
        fixed_variables: fixed_vars_str,
        error: String::new(),
    };
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

    let raw_fixed: HashMap<String, f64> = form
        .fixed_variables
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();

    let shapes = shape::Entity::find().all(&state.db).await.unwrap_or_default();
    let all_shapes: Vec<(i64, Vec<String>)> = shapes
        .iter()
        .map(|s| (s.id, s.variable_names_vec()))
        .collect();
    let cur_shape = shapes.iter().find(|s| s.id == form.shape_id);
    let shape_name = cur_shape.map(|s| s.name.clone()).unwrap_or_default();
    let shape_variables = cur_shape.map(|s| s.variable_names_vec()).unwrap_or_default();

    if form.name.trim().is_empty() {
        let material_name = material::Entity::find_by_id(form.material_id)
            .one(&state.db)
            .await
            .ok()
            .flatten()
            .map(|m| m.name)
            .unwrap_or_default();
        let page = ComponentEditModalPage {
            id,
            form_name: q.form_name(),
            name: form.name,
            shape_id: form.shape_id,
            shape_name,
            shape_variables,
            all_shapes,
            material_id: form.material_id,
            material_name,
            fixed_variables: form.fixed_variables.unwrap_or_default(),
            error: "Component name is required.".into(),
        };
        return html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response();
    }

    if form.shape_id <= 0 {
        let material_name = material::Entity::find_by_id(form.material_id)
            .one(&state.db)
            .await
            .ok()
            .flatten()
            .map(|m| m.name)
            .unwrap_or_default();
        let page = ComponentEditModalPage {
            id,
            form_name: q.form_name(),
            name: form.name,
            shape_id: form.shape_id,
            shape_name,
            shape_variables,
            all_shapes,
            material_id: form.material_id,
            material_name,
            fixed_variables: form.fixed_variables.unwrap_or_default(),
            error: "Please select a shape.".into(),
        };
        return html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response();
    }

    if form.material_id <= 0 {
        let page = ComponentEditModalPage {
            id,
            form_name: q.form_name(),
            name: form.name,
            shape_id: form.shape_id,
            shape_name,
            shape_variables,
            all_shapes,
            material_id: form.material_id,
            material_name: String::new(),
            fixed_variables: form.fixed_variables.unwrap_or_default(),
            error: "Please select a material.".into(),
        };
        return html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response();
    }

    for key in raw_fixed.keys() {
        let is_valid = if !shape_variables.is_empty() {
            shape_variables.contains(key)
        } else {
            is_allowed_variable_name(key)
        };
        if !is_valid {
            let material_name = material::Entity::find_by_id(form.material_id)
                .one(&state.db)
                .await
                .ok()
                .flatten()
                .map(|m| m.name)
                .unwrap_or_default();
            let page = ComponentEditModalPage {
                id,
                form_name: q.form_name(),
                name: form.name,
                shape_id: form.shape_id,
                shape_name: shape_name.clone(),
                shape_variables: shape_variables.clone(),
                all_shapes,
                material_id: form.material_id,
                material_name,
                fixed_variables: form.fixed_variables.unwrap_or_default(),
                error: format!(
                    "Variable '{}' is not valid for shape '{}'. Allowed variables: {}",
                    key,
                    shape_name,
                    if !shape_variables.is_empty() {
                        shape_variables.join(", ")
                    } else {
                        ALLOWED_VARIABLE_NAMES.join(", ")
                    }
                ),
            };
            return html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response();
        }
    }
    let fixed_vars = raw_fixed;

    let now = Utc::now();
    let mut am: component::ActiveModel = existing.into();
    am.updated_at = Set(Some(now));
    am.name = Set(form.name.trim().to_string());
    am.shape_id = Set(form.shape_id);
    am.material_id = Set(form.material_id);
    am.fixed_variables = Set(serde_json::to_value(&fixed_vars).unwrap_or_default());

    match am.update(&state.db).await {
        Ok(_) => respond_edit_modal_done::<ComponentEditModalKey>(
            &htmx,
            &ComponentDetailRouteTag::new(id).url(),
        ),
        Err(e) => {
            let material_name = material::Entity::find_by_id(form.material_id)
                .one(&state.db)
                .await
                .ok()
                .flatten()
                .map(|m| m.name)
                .unwrap_or_default();
            let page = ComponentEditModalPage {
                id,
                form_name: q.form_name(),
                name: form.name,
                shape_id: form.shape_id,
                shape_name,
                shape_variables,
                all_shapes,
                material_id: form.material_id,
                material_name,
                fixed_variables: form.fixed_variables.unwrap_or_default(),
                error: e.to_string(),
            };
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
// 3. SHAPES
// ==========================================

pub async fn shapes_list(
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    htmx: Htmx,
    uri: Uri,
) -> maud::Markup {
    let _ = crate::work_orders::seed::ensure_standard_seeds(&state.db).await;
    let shapes = shape::Entity::find()
        .order_by_asc(shape::Column::Name)
        .all(&state.db)
        .await
        .unwrap_or_default();

    let page = ShapeListPage {
        shapes,
        path_and_query: path_and_query(&uri),
    };
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    if htmx.targets::<ShapeTableKey>() {
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

pub async fn shape_detail(
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(shape) = shape::Entity::find_by_id(id).one(&state.db).await.unwrap_or(None) else {
        return Redirect::to(&WorkOrdersShapesRouteTag.url()).into_response();
    };

    let page = ShapeDetailPage { shape };
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx).into_response()
}


pub async fn shape_select(
    Cap(state): Cap<WorkOrdersState>,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<EntitySelectQuery>,
) -> maud::Markup {
    let _ = crate::work_orders::seed::ensure_standard_seeds(&state.db).await;
    let mut query = shape::Entity::find();
    if let Some(n) = q.name.as_deref().filter(|s| !s.trim().is_empty()) {
        query = query.filter(shape::Column::Name.contains(n));
    }
    let shapes = query.order_by_asc(shape::Column::Name).all(&state.db).await.unwrap_or_default();
    let page = ShapeSelectPage {
        shapes,
        target_input: q.target_input.unwrap_or_else(|| "shape_id".into()),
        path_and_query: uri.to_string(),
    };
    respond_picker_select::<ShapeSelectTableKey, ShapeSelectModalKey, _>(&htmx, &page)
}

pub use super::forms::{ShapeCreateForm, ShapeEditForm, ShapeForm};

pub async fn shape_create_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    Query(q): Query<ModalFormQuery>,
) -> maud::Markup {
    let page = ShapeCreateModalPage {
        form_name: q.form_name(),
        name: String::new(),
        variables: Vec::new(),
        openscad_code: String::new(),
        error: String::new(),
    };
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    html_built_page_with_slots(&page, &chrome, &slot_ctx)
}

pub async fn shape_create_post(
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    htmx: Htmx,
    Query(q): Query<ModalFormQuery>,
    HtmlFormBody(form): HtmlFormBody<ShapeForm>,
) -> Response {
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    let vars: Vec<String> = form
        .variables
        .into_iter()
        .flat_map(|s| {
            s.split(',')
                .map(|p| p.trim().to_string())
                .filter(|p| !p.is_empty())
                .collect::<Vec<_>>()
        })
        .collect();

    let now = Utc::now();
    let model = shape::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        name: Set(form.name.trim().to_string()),
        openscad_code: Set(form.openscad_code.trim().to_string()),
        variable_names: Set(serde_json::to_value(&vars).unwrap_or_default()),
    };

    match model.insert(&state.db).await {
        Ok(saved) => respond_create_modal_done::<ShapeCreateModalKey>(
            &htmx,
            &q.refresh_table(),
            &ShapeDetailRouteTag::new(saved.id).url(),
        ),
        Err(e) => {
            let page = ShapeCreateModalPage {
                form_name: q.form_name(),
                name: form.name,
                variables: vars,
                openscad_code: form.openscad_code,
                error: e.to_string(),
            };
            html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response()
        }
    }
}

pub async fn shape_edit_get(
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    Query(q): Query<ModalFormQuery>,
    Path(id): Path<i64>,
) -> Response {
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    let s = match shape::Entity::find_by_id(id).one(&state.db).await {
        Ok(Some(shp)) => shp,
        _ => return Redirect::to(&WorkOrdersShapesRouteTag.url()).into_response(),
    };
    let vars = s.variable_names_vec();
    let page = ShapeEditModalPage {
        id,
        form_name: q.form_name(),
        name: s.name,
        openscad_code: s.openscad_code,
        variables: vars,
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response()
}

pub async fn shape_edit_post(
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    htmx: Htmx,
    Query(q): Query<ModalFormQuery>,
    Path(id): Path<i64>,
    HtmlFormBody(form): HtmlFormBody<ShapeForm>,
) -> impl IntoResponse {
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    let existing = match shape::Entity::find_by_id(id).one(&state.db).await {
        Ok(Some(s)) => s,
        _ => return Redirect::to(&WorkOrdersShapesRouteTag.url()).into_response(),
    };

    let var_names: Vec<String> = form
        .variables
        .into_iter()
        .flat_map(|s| {
            s.split(',')
                .map(|p| p.trim().to_string())
                .filter(|p| !p.is_empty())
                .collect::<Vec<_>>()
        })
        .collect();

    let now = Utc::now();
    let mut am: shape::ActiveModel = existing.into();
    am.updated_at = Set(Some(now));
    am.name = Set(form.name.trim().to_string());
    am.openscad_code = Set(form.openscad_code.trim().to_string());
    am.variable_names = Set(serde_json::to_value(&var_names).unwrap_or_default());

    match am.update(&state.db).await {
        Ok(_) => respond_edit_modal_done::<ShapeEditModalKey>(
            &htmx,
            &ShapeDetailRouteTag::new(id).url(),
        ),
        Err(e) => {
            let page = ShapeEditModalPage {
                id,
                form_name: q.form_name(),
                name: form.name,
                openscad_code: form.openscad_code,
                variables: var_names,
                error: e.to_string(),
            };
            html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response()
        }
    }
}

pub async fn shape_delete_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    Path(id): Path<i64>,
) -> maud::Markup {
    let page = ConfirmDeleteModalPage {
        modal_uid: ShapeDeleteModalKey::ID.to_string(),
        title: "Delete Shape".into(),
        message: "Are you sure you want to delete this shape?".into(),
        post_url: ShapeDeletePostRouteTag::new(id).url(),
        error: String::new(),
    };
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    html_built_page_with_slots(&page, &chrome, &slot_ctx)
}

pub async fn shape_delete_post(
    Cap(state): Cap<WorkOrdersState>,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let _ = shape::Entity::delete_by_id(id).exec(&state.db).await;
    htmx.redirect(&WorkOrdersShapesRouteTag.url())
}

// ==========================================
// 4. MATERIALS
// ==========================================

pub async fn materials_list(
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    htmx: Htmx,
    uri: Uri,
) -> maud::Markup {
    let mats = material::Entity::find()
        .order_by_asc(material::Column::Name)
        .all(&state.db)
        .await
        .unwrap_or_default();

    let mut result = Vec::with_capacity(mats.len());
    for m in mats {
        let latest_rate = material_rate::Entity::find()
            .filter(material_rate::Column::MaterialId.eq(m.id))
            .order_by_desc(material_rate::Column::Datetime)
            .order_by_desc(material_rate::Column::Id)
            .one(&state.db)
            .await
            .unwrap_or(None);
        result.push((m, latest_rate));
    }

    let page = MaterialListPage {
        materials: result,
        path_and_query: path_and_query(&uri),
    };
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    if htmx.targets::<MaterialTableKey>() {
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

pub async fn material_detail(
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(mat) = material::Entity::find_by_id(id).one(&state.db).await.unwrap_or(None) else {
        return Redirect::to(&WorkOrdersMaterialsRouteTag.url()).into_response();
    };

    let rates = material_rate::Entity::find()
        .filter(material_rate::Column::MaterialId.eq(mat.id))
        .order_by_desc(material_rate::Column::Datetime)
        .all(&state.db)
        .await
        .unwrap_or_default();

    let page = MaterialDetailPage {
        material: mat,
        rates,
    };
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx).into_response()
}

pub async fn material_select(
    Cap(state): Cap<WorkOrdersState>,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<EntitySelectQuery>,
) -> maud::Markup {
    let _ = crate::work_orders::seed::ensure_standard_seeds(&state.db).await;
    let mut query = material::Entity::find();
    if let Some(n) = q.name.as_deref().filter(|s| !s.trim().is_empty()) {
        query = query.filter(material::Column::Name.contains(n));
    }
    let materials = query.order_by_asc(material::Column::Name).all(&state.db).await.unwrap_or_default();
    let page = MaterialSelectPage {
        materials,
        target_input: q.target_input.unwrap_or_else(|| "material_id".into()),
        path_and_query: uri.to_string(),
    };
    respond_picker_select::<MaterialSelectTableKey, MaterialSelectModalKey, _>(&htmx, &page)
}

pub async fn material_create_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    Query(q): Query<ModalFormQuery>,
) -> maud::Markup {
    let page = MaterialCreateModalPage {
        form_name: q.form_name(),
        error: String::new(),
    };
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    html_built_page_with_slots(&page, &chrome, &slot_ctx)
}

#[derive(Deserialize)]
pub struct MaterialCreateForm {
    pub name: String,
    pub density: f64,
}

pub async fn material_create_post(
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    htmx: Htmx,
    Query(q): Query<ModalFormQuery>,
    Form(form): Form<MaterialCreateForm>,
) -> Response {
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    let now = Utc::now();
    let model = material::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        name: Set(form.name.trim().to_string()),
        density: Set(form.density),
    };

    match model.insert(&state.db).await {
        Ok(saved) => respond_create_modal_done::<MaterialCreateModalKey>(
            &htmx,
            &q.refresh_table(),
            &MaterialDetailRouteTag::new(saved.id).url(),
        ),
        Err(e) => {
            let page = MaterialCreateModalPage {
                form_name: q.form_name(),
                error: e.to_string(),
            };
            html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response()
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct MaterialEditForm {
    pub name: String,
    pub density: f64,
}

pub async fn material_edit_get(
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    Query(q): Query<ModalFormQuery>,
    Path(id): Path<i64>,
) -> Response {
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    let m = match material::Entity::find_by_id(id).one(&state.db).await {
        Ok(Some(mat)) => mat,
        _ => return Redirect::to(&WorkOrdersMaterialsRouteTag.url()).into_response(),
    };
    let page = MaterialEditModalPage {
        id,
        form_name: q.form_name(),
        name: m.name,
        density: m.density,
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response()
}

pub async fn material_edit_post(
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    htmx: Htmx,
    Query(q): Query<ModalFormQuery>,
    Path(id): Path<i64>,
    Form(form): Form<MaterialEditForm>,
) -> impl IntoResponse {
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    let existing = match material::Entity::find_by_id(id).one(&state.db).await {
        Ok(Some(m)) => m,
        _ => return Redirect::to(&WorkOrdersMaterialsRouteTag.url()).into_response(),
    };

    let now = Utc::now();
    let mut am: material::ActiveModel = existing.into();
    am.updated_at = Set(Some(now));
    am.name = Set(form.name.trim().to_string());
    am.density = Set(form.density);

    match am.update(&state.db).await {
        Ok(_) => respond_edit_modal_done::<MaterialEditModalKey>(
            &htmx,
            &MaterialDetailRouteTag::new(id).url(),
        ),
        Err(e) => {
            let page = MaterialEditModalPage {
                id,
                form_name: q.form_name(),
                name: form.name,
                density: form.density,
                error: e.to_string(),
            };
            html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response()
        }
    }
}

pub async fn material_delete_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    Path(id): Path<i64>,
) -> maud::Markup {
    let page = ConfirmDeleteModalPage {
        modal_uid: MaterialDeleteModalKey::ID.to_string(),
        title: "Delete Material".into(),
        message: "Are you sure you want to delete this material?".into(),
        post_url: MaterialDeletePostRouteTag::new(id).url(),
        error: String::new(),
    };
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    html_built_page_with_slots(&page, &chrome, &slot_ctx)
}

pub async fn material_delete_post(
    Cap(state): Cap<WorkOrdersState>,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let _ = material::Entity::delete_by_id(id).exec(&state.db).await;
    htmx.redirect(&WorkOrdersMaterialsRouteTag.url())
}

// ==========================================
// 5. MATERIAL RATES
// ==========================================

pub async fn rates_list(
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    htmx: Htmx,
    uri: Uri,
) -> maud::Markup {
    let rates = material_rate::Entity::find()
        .order_by_desc(material_rate::Column::Datetime)
        .all(&state.db)
        .await
        .unwrap_or_default();

    let mut result = Vec::with_capacity(rates.len());
    for r in rates {
        let mat = material::Entity::find_by_id(r.material_id).one(&state.db).await.unwrap_or(None);
        result.push((r, mat));
    }

    let page = MaterialRateListPage {
        rates: result,
        path_and_query: path_and_query(&uri),
    };
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    if htmx.targets::<MaterialRateTableKey>() {
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

pub async fn rate_create_get(
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    Query(q): Query<ModalFormQuery>,
) -> maud::Markup {
    let materials = material::Entity::find().order_by_asc(material::Column::Name).all(&state.db).await.unwrap_or_default();
    let page = MaterialRateCreateModalPage {
        form_name: q.form_name(),
        materials,
        error: String::new(),
    };
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    html_built_page_with_slots(&page, &chrome, &slot_ctx)
}

#[derive(Deserialize)]
pub struct RateCreateForm {
    pub material_id: i64,
    pub rate: Decimal,
}

pub async fn rate_create_post(
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    htmx: Htmx,
    Query(q): Query<ModalFormQuery>,
    Form(form): Form<RateCreateForm>,
) -> Response {
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    let now = Utc::now();
    let model = material_rate::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        material_id: Set(form.material_id),
        rate_decimal: Set(form.rate),
        datetime: Set(now),
    };

    match model.insert(&state.db).await {
        Ok(_) => respond_create_modal_done::<MaterialRateCreateModalKey>(
            &htmx,
            &q.refresh_table(),
            &WorkOrdersRatesRouteTag.url(),
        ),
        Err(e) => {
            let materials = material::Entity::find().all(&state.db).await.unwrap_or_default();
            let page = MaterialRateCreateModalPage {
                form_name: q.form_name(),
                materials,
                error: e.to_string(),
            };
            html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response()
        }
    }
}

pub async fn rate_delete_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    Path(id): Path<i64>,
) -> maud::Markup {
    let page = ConfirmDeleteModalPage {
        modal_uid: MaterialRateDeleteModalKey::ID.to_string(),
        title: "Delete Material Rate".into(),
        message: "Are you sure you want to delete this material rate entry?".into(),
        post_url: MaterialRateDeletePostRouteTag::new(id).url(),
        error: String::new(),
    };
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    html_built_page_with_slots(&page, &chrome, &slot_ctx)
}

pub async fn rate_delete_post(
    Cap(state): Cap<WorkOrdersState>,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let _ = material_rate::Entity::delete_by_id(id).exec(&state.db).await;
    htmx.redirect(&WorkOrdersRatesRouteTag.url())
}

// ==========================================
// 6. MACHINES
// ==========================================

pub async fn machines_list(
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    htmx: Htmx,
    uri: Uri,
) -> maud::Markup {
    let machines = machine::Entity::find()
        .order_by_asc(machine::Column::Name)
        .all(&state.db)
        .await
        .unwrap_or_default();

    let page = MachineListPage {
        machines,
        path_and_query: path_and_query(&uri),
    };
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    if htmx.targets::<MachineTableKey>() {
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

pub async fn machine_detail(
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(machine) = machine::Entity::find_by_id(id).one(&state.db).await.unwrap_or(None) else {
        return Redirect::to(&WorkOrdersMachinesRouteTag.url()).into_response();
    };

    let page = MachineDetailPage { machine };
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx).into_response()
}

pub async fn machine_create_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    Query(q): Query<ModalFormQuery>,
) -> maud::Markup {
    let page = MachineCreateModalPage {
        form_name: q.form_name(),
        error: String::new(),
    };
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    html_built_page_with_slots(&page, &chrome, &slot_ctx)
}

#[derive(Deserialize)]
pub struct MachineCreateForm {
    pub name: String,
    pub rate: Decimal,
}

pub async fn machine_create_post(
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    htmx: Htmx,
    Query(q): Query<ModalFormQuery>,
    Form(form): Form<MachineCreateForm>,
) -> Response {
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    let now = Utc::now();
    let model = machine::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        name: Set(form.name.trim().to_string()),
        rate_decimal: Set(form.rate),
    };

    match model.insert(&state.db).await {
        Ok(saved) => respond_create_modal_done::<MachineCreateModalKey>(
            &htmx,
            &q.refresh_table(),
            &MachineDetailRouteTag::new(saved.id).url(),
        ),
        Err(e) => {
            let page = MachineCreateModalPage {
                form_name: q.form_name(),
                error: e.to_string(),
            };
            html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response()
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct MachineEditForm {
    pub name: String,
    pub rate: f64,
}

pub async fn machine_edit_get(
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    Query(q): Query<ModalFormQuery>,
    Path(id): Path<i64>,
) -> Response {
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    let m = match machine::Entity::find_by_id(id).one(&state.db).await {
        Ok(Some(mach)) => mach,
        _ => return Redirect::to(&WorkOrdersMachinesRouteTag.url()).into_response(),
    };
    use rust_decimal::prelude::ToPrimitive;
    let rate_f64 = m.rate_decimal.to_f64().unwrap_or(0.0);
    let page = MachineEditModalPage {
        id,
        form_name: q.form_name(),
        name: m.name,
        rate: rate_f64,
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response()
}

pub async fn machine_edit_post(
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    htmx: Htmx,
    Query(q): Query<ModalFormQuery>,
    Path(id): Path<i64>,
    Form(form): Form<MachineEditForm>,
) -> impl IntoResponse {
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    let existing = match machine::Entity::find_by_id(id).one(&state.db).await {
        Ok(Some(m)) => m,
        _ => return Redirect::to(&WorkOrdersMachinesRouteTag.url()).into_response(),
    };

    use rust_decimal::prelude::FromPrimitive;
    let rate_dec = Decimal::from_f64(form.rate).unwrap_or(existing.rate_decimal);

    let now = Utc::now();
    let mut am: machine::ActiveModel = existing.into();
    am.updated_at = Set(Some(now));
    am.name = Set(form.name.trim().to_string());
    am.rate_decimal = Set(rate_dec);

    match am.update(&state.db).await {
        Ok(_) => respond_edit_modal_done::<MachineEditModalKey>(
            &htmx,
            &MachineDetailRouteTag::new(id).url(),
        ),
        Err(e) => {
            let page = MachineEditModalPage {
                id,
                form_name: q.form_name(),
                name: form.name,
                rate: form.rate,
                error: e.to_string(),
            };
            html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response()
        }
    }
}

pub async fn machine_delete_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    Path(id): Path<i64>,
) -> maud::Markup {
    let page = ConfirmDeleteModalPage {
        modal_uid: MachineDeleteModalKey::ID.to_string(),
        title: "Delete Machine".into(),
        message: "Are you sure you want to delete this machine?".into(),
        post_url: MachineDeletePostRouteTag::new(id).url(),
        error: String::new(),
    };
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    html_built_page_with_slots(&page, &chrome, &slot_ctx)
}

pub async fn machine_delete_post(
    Cap(state): Cap<WorkOrdersState>,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let _ = machine::Entity::delete_by_id(id).exec(&state.db).await;
    htmx.redirect(&WorkOrdersMachinesRouteTag.url())
}

// ==========================================
// 7. PROFORMA INVOICES
// ==========================================

pub async fn invoices_list(
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    htmx: Htmx,
    uri: Uri,
) -> maud::Markup {
    let invoices = proforma_invoice::Entity::find()
        .order_by_desc(proforma_invoice::Column::Date)
        .order_by_desc(proforma_invoice::Column::Id)
        .all(&state.db)
        .await
        .unwrap_or_default();

    let mut customer_names = Vec::with_capacity(invoices.len());
    let mut grand_totals = Vec::with_capacity(invoices.len());
    for inv in &invoices {
        let name = lariv_rs::plugins::customer::entities::customer::Entity::find_by_id(inv.customer_id)
            .one(&state.db).await.ok().flatten()
            .map(|c| c.name)
            .unwrap_or_else(|| format!("Customer #{}", inv.customer_id));
        customer_names.push(name);
        let mlines = proforma_invoice_material_line::Entity::find()
            .filter(proforma_invoice_material_line::Column::InvoiceId.eq(inv.id))
            .all(&state.db).await.unwrap_or_default();
        let mlmach = proforma_invoice_machine_line::Entity::find()
            .filter(proforma_invoice_machine_line::Column::InvoiceId.eq(inv.id))
            .all(&state.db).await.unwrap_or_default();
        grand_totals.push(inv.grand_total(&mlines, &mlmach));
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
    let Some(inv) = proforma_invoice::Entity::find_by_id(id).one(&state.db).await.unwrap_or(None) else {
        return Redirect::to(&WorkOrdersInvoicesRouteTag.url()).into_response();
    };

    let machine_lines = proforma_invoice_machine_line::Entity::find()
        .filter(proforma_invoice_machine_line::Column::InvoiceId.eq(inv.id))
        .all(&state.db)
        .await
        .unwrap_or_default();

    let material_lines = proforma_invoice_material_line::Entity::find()
        .filter(proforma_invoice_material_line::Column::InvoiceId.eq(inv.id))
        .all(&state.db)
        .await
        .unwrap_or_default();

    let grand_total = inv.grand_total(&material_lines, &machine_lines);

    let customer_name = lariv_rs::plugins::customer::entities::customer::Entity::find_by_id(inv.customer_id)
        .one(&state.db).await.ok().flatten()
        .map(|c| c.name)
        .unwrap_or_else(|| format!("Customer #{}", inv.customer_id));

    let work_order_number = if let Some(wo_id) = inv.work_order_id {
        work_order::Entity::find_by_id(wo_id).one(&state.db).await.ok().flatten()
            .map(|wo| wo.order_number)
            .unwrap_or_default()
    } else {
        String::new()
    };

    let page = InvoiceDetailPage {
        invoice: inv,
        machine_lines,
        material_lines,
        grand_total,
        customer_name,
        work_order_number,
    };
    let slot_ctx = auth.0.as_ref().map(SlotCtx::from_auth).unwrap_or_default();
    html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx).into_response()
}

pub async fn invoice_create_get(
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    auth: OptionalAuth,
    Query(q): Query<ModalFormQuery>,
) -> maud::Markup {
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let materials_json = fetch_materials_json(&state.db).await;
    let machines_json = fetch_machines_json(&state.db).await;

    let page = InvoiceCreateModalPage {
        form_name: q.form_name(),
        date: today,
        customer_name: String::new(),
        work_order_name: String::new(),
        material_lines_json: "[]".into(),
        machine_lines_json: "[]".into(),
        materials_json,
        machines_json,
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
    #[serde(alias = "customer_id", alias = "CustomerID", default, deserialize_with = "i64_from_str_or_zero")]
    pub customer_id: i64,
    #[serde(alias = "work_order_id", alias = "WorkOrderID", default, deserialize_with = "opt_i64_from_str")]
    pub work_order_id: Option<i64>,
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
        let customer_name = String::new();
        let work_order_name = if let Some(wo_id) = form.work_order_id {
            work_order::Entity::find_by_id(wo_id).one(&state.db).await.ok().flatten()
                .map(|wo| wo.order_number).unwrap_or_default()
        } else {
            String::new()
        };
        let materials_json = fetch_materials_json(&state.db).await;
        let machines_json = fetch_machines_json(&state.db).await;
        let page = InvoiceCreateModalPage {
            form_name: q.form_name(),
            date: form.date.clone(),
            customer_name,
            work_order_name,
            material_lines_json: form.material_lines.clone().unwrap_or_default(),
            machine_lines_json: form.machine_lines.clone().unwrap_or_default(),
            materials_json,
            machines_json,
            error: "Please select a customer.".into(),
        };
        return html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response();
    }

    let date = match chrono::NaiveDate::parse_from_str(form.date.trim(), "%Y-%m-%d") {
        Ok(d) => d,
        Err(_) => {
            let materials_json = fetch_materials_json(&state.db).await;
            let machines_json = fetch_machines_json(&state.db).await;
            let page = InvoiceCreateModalPage {
                form_name: q.form_name(),
                date: form.date.clone(),
                customer_name: String::new(),
                work_order_name: String::new(),
                material_lines_json: form.material_lines.clone().unwrap_or_default(),
                machine_lines_json: form.machine_lines.clone().unwrap_or_default(),
                materials_json,
                machines_json,
                error: "Invalid date format. Use YYYY-MM-DD.".into(),
            };
            return html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response();
        }
    };

    let now = Utc::now();
    let model = proforma_invoice::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        date: Set(date),
        customer_id: Set(form.customer_id),
        invoice_number: Set(form.invoice_number.trim().to_string()),
        work_order_id: Set(form.work_order_id),
    };

    match model.insert(&state.db).await {
        Ok(saved) => {
            sync_invoice_lines(&state.db, saved.id, form.material_lines, form.machine_lines, now).await;
            respond_create_modal_done::<InvoiceCreateModalKey>(
                &htmx,
                &q.refresh_table(),
                &InvoiceDetailRouteTag::new(saved.id).url(),
            )
        }
        Err(e) => {
            let customer_name = lariv_rs::plugins::customer::entities::customer::Entity::find_by_id(form.customer_id)
                .one(&state.db).await.ok().flatten().map(|c| c.name).unwrap_or_default();
            let work_order_name = if let Some(wo_id) = form.work_order_id {
                work_order::Entity::find_by_id(wo_id).one(&state.db).await.ok().flatten()
                    .map(|wo| wo.order_number).unwrap_or_default()
            } else {
                String::new()
            };
            let materials_json = fetch_materials_json(&state.db).await;
            let machines_json = fetch_machines_json(&state.db).await;
            let page = InvoiceCreateModalPage {
                form_name: q.form_name(),
                date: form.date.clone(),
                customer_name,
                work_order_name,
                material_lines_json: form.material_lines.clone().unwrap_or_default(),
                machine_lines_json: form.machine_lines.clone().unwrap_or_default(),
                materials_json,
                machines_json,
                error: e.to_string(),
            };
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
    let inv = match proforma_invoice::Entity::find_by_id(id).one(&state.db).await {
        Ok(Some(i)) => i,
        _ => return Redirect::to(&WorkOrdersInvoicesRouteTag.url()).into_response(),
    };

    let customer_name = lariv_rs::plugins::customer::entities::customer::Entity::find_by_id(inv.customer_id)
        .one(&state.db).await.ok().flatten()
        .map(|c| c.name)
        .unwrap_or_else(|| format!("Customer #{}", inv.customer_id));

    let work_order_name = if let Some(wo_id) = inv.work_order_id {
        work_order::Entity::find_by_id(wo_id).one(&state.db).await.ok().flatten()
            .map(|wo| wo.order_number)
            .unwrap_or_default()
    } else {
        String::new()
    };

    let material_lines_json = invoice_material_lines_json(&state.db, inv.id).await;
    let machine_lines_json = invoice_machine_lines_json(&state.db, inv.id).await;
    let materials_json = fetch_materials_json(&state.db).await;
    let machines_json = fetch_machines_json(&state.db).await;

    let page = InvoiceEditModalPage {
        id,
        form_name: q.form_name(),
        invoice_number: inv.invoice_number,
        date: inv.date.to_string(),
        customer_id: inv.customer_id,
        customer_name,
        work_order_id: inv.work_order_id,
        work_order_name,
        material_lines_json,
        machine_lines_json,
        materials_json,
        machines_json,
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
    let existing = match proforma_invoice::Entity::find_by_id(id).one(&state.db).await {
        Ok(Some(i)) => i,
        _ => return Redirect::to(&WorkOrdersInvoicesRouteTag.url()).into_response(),
    };

    let date = chrono::NaiveDate::parse_from_str(form.date.trim(), "%Y-%m-%d")
        .unwrap_or(existing.date);

    let now = Utc::now();
    let mut am: proforma_invoice::ActiveModel = existing.into();
    am.updated_at = Set(Some(now));
    am.invoice_number = Set(form.invoice_number.trim().to_string());
    am.date = Set(date);
    am.customer_id = Set(form.customer_id);
    am.work_order_id = Set(form.work_order_id);

    match am.update(&state.db).await {
        Ok(_) => {
            sync_invoice_lines(&state.db, id, form.material_lines, form.machine_lines, now).await;
            respond_edit_modal_done::<InvoiceEditModalKey>(
                &htmx,
                &InvoiceDetailRouteTag::new(id).url(),
            )
        }
        Err(e) => {
            let customer_name = lariv_rs::plugins::customer::entities::customer::Entity::find_by_id(form.customer_id)
                .one(&state.db).await.ok().flatten().map(|c| c.name).unwrap_or_default();
            let work_order_name = if let Some(wo_id) = form.work_order_id {
                work_order::Entity::find_by_id(wo_id).one(&state.db).await.ok().flatten()
                    .map(|wo| wo.order_number).unwrap_or_default()
            } else {
                String::new()
            };
            let materials_json = fetch_materials_json(&state.db).await;
            let machines_json = fetch_machines_json(&state.db).await;
            let page = InvoiceEditModalPage {
                id,
                form_name: q.form_name(),
                invoice_number: form.invoice_number.clone(),
                date: form.date.clone(),
                customer_id: form.customer_id,
                customer_name,
                work_order_id: form.work_order_id,
                work_order_name,
                material_lines_json: form.material_lines.clone().unwrap_or_default(),
                machine_lines_json: form.machine_lines.clone().unwrap_or_default(),
                materials_json,
                machines_json,
                error: e.to_string(),
            };
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
        title: "Delete Proforma Invoice".into(),
        message: "Are you sure you want to delete this proforma invoice?".into(),
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
    let _ = proforma_invoice::Entity::delete_by_id(id).exec(&state.db).await;
    htmx.redirect(&WorkOrdersInvoicesRouteTag.url())
}

// ==========================================
// 8. CAD API
// ==========================================

#[derive(Debug, Deserialize)]
pub struct CalculateRequest {
    pub shape_id: i64,
    pub material_id: i64,
    pub variables: HashMap<String, f64>,
}

#[derive(Debug, Serialize)]
pub struct CalculateResponse {
    pub volume_m3: f64,
    pub weight_kg: f64,
    pub cost_inr: f64,
    pub error: Option<String>,
}

pub async fn calculate_api(
    Cap(state): Cap<WorkOrdersState>,
    Json(payload): Json<CalculateRequest>,
) -> impl IntoResponse {
    let shape_res = shape::Entity::find_by_id(payload.shape_id).one(&state.db).await;
    let shape = match shape_res {
        Ok(Some(s)) => s,
        Ok(None) => {
            return Json(CalculateResponse {
                volume_m3: 0.0,
                weight_kg: 0.0,
                cost_inr: 0.0,
                error: Some(format!("Shape #{} not found", payload.shape_id)),
            });
        }
        Err(e) => {
            return Json(CalculateResponse {
                volume_m3: 0.0,
                weight_kg: 0.0,
                cost_inr: 0.0,
                error: Some(e.to_string()),
            });
        }
    };

    let material_res = material::Entity::find_by_id(payload.material_id).one(&state.db).await;
    let mat = match material_res {
        Ok(Some(m)) => m,
        Ok(None) => {
            return Json(CalculateResponse {
                volume_m3: 0.0,
                weight_kg: 0.0,
                cost_inr: 0.0,
                error: Some(format!("Material #{} not found", payload.material_id)),
            });
        }
        Err(e) => {
            return Json(CalculateResponse {
                volume_m3: 0.0,
                weight_kg: 0.0,
                cost_inr: 0.0,
                error: Some(e.to_string()),
            });
        }
    };

    let volume_res = shape.get_volume_async(&payload.variables).await;
    let volume = match volume_res {
        Ok(v) => v,
        Err(e) => {
            return Json(CalculateResponse {
                volume_m3: 0.0,
                weight_kg: 0.0,
                cost_inr: 0.0,
                error: Some(format!("CAD calculation failed: {}", e)),
            });
        }
    };

    let weight = volume * mat.density;
    let rate_opt = material_rate::Entity::find()
        .filter(material_rate::Column::MaterialId.eq(mat.id))
        .order_by_desc(material_rate::Column::Datetime)
        .order_by_desc(material_rate::Column::Id)
        .one(&state.db)
        .await
        .unwrap_or(None);

    let rate = rate_opt.map(|r| r.rate()).unwrap_or(0.0);
    let cost = weight * rate;

    Json(CalculateResponse {
        volume_m3: volume,
        weight_kg: weight,
        cost_inr: cost,
        error: None,
    })
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
            tracing::error!("work orders pdf: {msg}");
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

fn render_preview_modal(pdf_url: &str, error: Option<&str>) -> Markup {
    if let Some(err) = error {
        return modal_keyed::<WorkOrdersPdfPreviewModalKey>(
            "max-w-2xl",
            html! {
                h3 class="text-lg font-semibold mb-2" { "Work Orders PDF preview failed" }
                p class="text-error whitespace-pre-wrap" { (err) }
            },
        );
    }
    modal_keyed::<WorkOrdersPdfPreviewModalKey>(
        "max-w-6xl w-[95vw]",
        html! {
            h3 class="text-lg font-semibold mb-3" { "Work Orders PDF preview (sample data)" }
            iframe
                src=(pdf_url)
                class="w-full h-[75vh] border border-base-300 rounded bg-white"
                title="Work Orders PDF preview" {}
        },
    )
}

fn prefs_page(prefs: WorkOrdersPreferences, error: String) -> WorkOrdersPreferencesPage {
    WorkOrdersPreferencesPage {
        draft_work_order_pdf_template: prefs
            .draft_work_order_pdf_template
            .unwrap_or_default(),
        proforma_invoice_pdf_template: prefs
            .proforma_invoice_pdf_template
            .unwrap_or_default(),
        error,
    }
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
            let page = prefs_page(empty_preferences(), format!("Failed to load preferences: {e}"));
            return html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx).into_response();
        }
    };
    let page = prefs_page(prefs, String::new());
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
    let prefs = WorkOrdersPreferences {
        id: 1,
        created_at: None,
        updated_at: None,
        draft_work_order_pdf_template: Some(form.draft_work_order_pdf_template),
        proforma_invoice_pdf_template: Some(form.proforma_invoice_pdf_template),
    };
    match save_preferences(&state.db, prefs.clone()).await {
        Ok(_) => htmx.redirect(&WorkOrdersPrefsGetRouteTag.url()),
        Err(e) => {
            let page = prefs_page(prefs, format!("Failed to save preferences: {e}"));
            html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx).into_response()
        }
    }
}

/// HTTP handler: `get /work-orders/orders/{id}/pdf`.
pub async fn work_order_pdf(
    Cap(state): Cap<WorkOrdersState>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
) -> Response {
    if !require_superuser(&ctx) {
        return StatusCode::FORBIDDEN.into_response();
    }
    match pdf::render_work_order_pdf(&state.db, id, &ctx.timezone).await {
        Ok(result) => pdf_ok_response(result),
        Err(e) => pdf_error_response(e),
    }
}

/// HTTP handler: `get /work-orders/invoices/{id}/pdf`.
pub async fn invoice_pdf(
    Cap(state): Cap<WorkOrdersState>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
) -> Response {
    if !require_superuser(&ctx) {
        return StatusCode::FORBIDDEN.into_response();
    }
    match pdf::render_proforma_invoice_pdf(&state.db, id, &ctx.timezone).await {
        Ok(result) => pdf_ok_response(result),
        Err(e) => pdf_error_response(e),
    }
}

/// HTTP handler: `post /work-orders/pdf/preview/work-order`.
pub async fn work_order_pdf_preview_post(
    Cap(state): Cap<WorkOrdersState>,
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
    match pdf::render_work_order_pdf_preview(&state.db, template, &ctx.timezone).await {
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
    RequireAuth(ctx): RequireAuth,
    HtmlFormBody(form): HtmlFormBody<WorkOrdersPreferencesForm>,
) -> Markup {
    if !require_superuser(&ctx) {
        return render_preview_modal("", Some("Forbidden"));
    }
    cleanup_stale_previews(3600);
    let template = if form.proforma_invoice_pdf_template.trim().is_empty() {
        None
    } else {
        Some(form.proforma_invoice_pdf_template.as_str())
    };
    match pdf::render_proforma_invoice_pdf_preview(&state.db, template).await {
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
pub async fn preview_pdf_get(
    RequireAuth(ctx): RequireAuth,
    Path(token): Path<String>,
) -> Response {
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
        let json = r#"[{"id":null,"material_id":1,"name":"MS","qty":"10","rate":"250","amount":2500}]"#;
        let lines: Vec<crate::work_orders::forms::InvoiceMaterialLineInput> =
            serde_json::from_str(json).expect("widget snake_case json must parse");
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].material_id, Some(1));
        assert_eq!(lines[0].name, "MS");
        assert_eq!(lines[0].qty, "10");
        assert_eq!(lines[0].rate, "250");
    }

    #[test]
    fn invoice_machine_lines_parse_widget_snake_case_json() {
        let json = r#"[{"id":null,"machine_id":1,"name":"CNC","duration":"2h 30m","rate":"950","amount":2375}]"#;
        let lines: Vec<crate::work_orders::forms::InvoiceMachineLineInput> =
            serde_json::from_str(json).expect("widget snake_case json must parse");
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].machine_id, Some(1));
        assert_eq!(lines[0].duration, "2h 30m");
        assert_eq!(lines[0].rate, "950");
    }

    #[test]
    fn invoice_line_inputs_still_accept_pascal_case_json() {
        let material: Vec<crate::work_orders::forms::InvoiceMaterialLineInput> =
            serde_json::from_str(r#"[{"MaterialId":2,"Name":"Aluminium","Qty":"5","Rate":"500"}]"#)
                .expect("pascal case json must parse");
        assert_eq!(material[0].material_id, Some(2));
        assert_eq!(material[0].qty, "5");

        let machine: Vec<crate::work_orders::forms::InvoiceMachineLineInput> =
            serde_json::from_str(r#"[{"MachineId":3,"Duration":"1h","Rate":"300"}]"#)
                .expect("pascal case json must parse");
        assert_eq!(machine[0].machine_id, Some(3));
        assert_eq!(machine[0].duration, "1h");
    }
}
