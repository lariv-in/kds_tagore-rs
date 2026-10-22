//! One service per machine.
//!
//! The service stores the machine's name, variable schema, and cost formula.
//! `machinery_machines.product_id` and quotation machine lines store that service id.

use std::collections::HashMap;

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter,
};
use uuid::Uuid;

use lariv_plugin_finance_services::entities::service;

use crate::entities::machine;

fn formula_text(formula: &str) -> String {
    let trimmed = formula.trim();
    if trimmed.is_empty() {
        "0".into()
    } else {
        trimmed.to_string()
    }
}

fn schema_text(variables: &serde_json::Value) -> String {
    match variables {
        serde_json::Value::Object(_) => variables.to_string(),
        serde_json::Value::String(s) if !s.trim().is_empty() => s.trim().to_string(),
        _ => "{}".into(),
    }
}

fn display_name(name: &str) -> String {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        "Machine".into()
    } else {
        trimmed.to_string()
    }
}

async fn insert_service<C: ConnectionTrait>(
    db: &C,
    name: String,
    variables: String,
    formula: String,
) -> Result<i64, String> {
    let now = Utc::now();
    let saved = service::ActiveModel {
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        reference: Set(Some(format!("kds-{}", Uuid::new_v4()))),
        remarks: Set(None),
        name: Set(name),
        sac_code: Set(0),
        variables: Set(variables),
        base_price_formula: Set(formula.clone()),
        sales_price_formula: Set(formula),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(|e| e.to_string())?;
    Ok(saved.id)
}

/// Create or update the service that stores a machine's schema and cost formula.
pub async fn save_machine_product<C: ConnectionTrait>(
    db: &C,
    service_id: i64,
    name: &str,
    variables: &serde_json::Value,
    cost_formula: &str,
) -> Result<i64, String> {
    let name = display_name(name);
    let variables = schema_text(variables);
    let formula = formula_text(cost_formula);
    if service_id > 0
        && let Some(existing) = service::Entity::find_by_id(service_id)
            .one(db)
            .await
            .map_err(|e| e.to_string())?
    {
        let mut am: service::ActiveModel = existing.into();
        am.updated_at = Set(Some(Utc::now()));
        am.name = Set(name);
        am.variables = Set(variables);
        am.base_price_formula = Set(formula.clone());
        am.sales_price_formula = Set(formula);
        let saved = am.update(db).await.map_err(|e| e.to_string())?;
        return Ok(saved.id);
    }
    insert_service(db, name, variables, formula).await
}

/// Service linked to this machine, creating one when the link is missing.
pub async fn ensure_machine_product<C: ConnectionTrait>(
    db: &C,
    machine: &machine::Model,
) -> Result<i64, String> {
    if machine.product_id > 0 {
        let exists = service::Entity::find_by_id(machine.product_id)
            .one(db)
            .await
            .map_err(|e| e.to_string())?
            .is_some();
        if exists {
            return Ok(machine.product_id);
        }
    }
    let id = save_machine_product(
        db,
        0,
        &machine.name,
        &machine.variables,
        &machine.cost_formula,
    )
    .await?;
    let mut am: machine::ActiveModel = machine.clone().into();
    am.product_id = Set(id);
    am.update(db).await.map_err(|e| e.to_string())?;
    Ok(id)
}

pub async fn machine_product_id<C: ConnectionTrait>(
    db: &C,
    machine_id: i64,
) -> Result<i64, String> {
    let machine = machine::Entity::find_by_id(machine_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Machine #{machine_id} not found"))?;
    ensure_machine_product(db, &machine).await
}

pub async fn service_remarks_by_id<C: ConnectionTrait>(
    db: &C,
    ids: &[i64],
) -> HashMap<i64, String> {
    let ids: Vec<i64> = ids.iter().copied().filter(|id| *id > 0).collect();
    if ids.is_empty() {
        return HashMap::new();
    }
    let services = service::Entity::find()
        .filter(service::Column::Id.is_in(ids))
        .all(db)
        .await
        .unwrap_or_default();
    services
        .into_iter()
        .map(|service| (service.id, service.remarks.unwrap_or_default()))
        .collect()
}
