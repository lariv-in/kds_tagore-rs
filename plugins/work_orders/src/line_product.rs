//! Catalog rows backing quotation and work-order lines.
//!
//! A component owns one goods product. Each machine owns one service.
//! Variable schemas and price formulas live on that row; the line keeps the
//! values entered for that job.

use std::collections::HashMap;

use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter,
};
use uuid::Uuid;

use lariv_plugin_finance_products::entities::product;
use lariv_plugin_inventory::entities::stock;

const COMPONENT_STOCK_NAME: &str = "Components";
const COMPONENT_STOCK_QTY_TYPE: &str = "quantity";
const COMPONENT_STOCK_QTY_UNIT: &str = "Number";

async fn component_stock_id<C: ConnectionTrait>(db: &C) -> Result<i64, String> {
    if let Some(existing) = stock::Entity::find()
        .filter(stock::Column::Name.eq(COMPONENT_STOCK_NAME))
        .filter(stock::Column::QtyType.eq(COMPONENT_STOCK_QTY_TYPE))
        .one(db)
        .await
        .map_err(|e| e.to_string())?
    {
        return Ok(existing.id);
    }
    let now = Utc::now();
    let saved = stock::ActiveModel {
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        name: Set(COMPONENT_STOCK_NAME.into()),
        qty_type: Set(COMPONENT_STOCK_QTY_TYPE.into()),
        qty_unit: Set(COMPONENT_STOCK_QTY_UNIT.into()),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(|e| e.to_string())?;
    Ok(saved.id)
}

fn formula_text(formula: &str, fallback: Decimal) -> String {
    let trimmed = formula.trim();
    if !trimmed.is_empty() {
        return trimmed.to_string();
    }
    let text = fallback.normalize().to_string();
    if text.is_empty() { "0".into() } else { text }
}

fn schema_text(variables: &serde_json::Value) -> String {
    match variables {
        serde_json::Value::Object(_) => variables.to_string(),
        serde_json::Value::String(s) if !s.trim().is_empty() => s.trim().to_string(),
        _ => "{}".into(),
    }
}

fn display_name(name: &str, fallback: &str) -> String {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.to_string()
    }
}

async fn insert_product<C: ConnectionTrait>(
    db: &C,
    name: String,
    variables: String,
    formula: String,
) -> Result<i64, String> {
    let now = Utc::now();
    let stock_id = component_stock_id(db).await?;
    let saved = product::ActiveModel {
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        stock_id: Set(stock_id),
        reference: Set(Some(format!("kds-{}", Uuid::new_v4()))),
        remarks: Set(None),
        name: Set(name),
        hsn_code: Set(0),
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

async fn write_product<C: ConnectionTrait>(
    db: &C,
    product_id: i64,
    name: String,
    variables: String,
    formula: String,
) -> Result<i64, String> {
    if product_id > 0
        && let Some(existing) = product::Entity::find_by_id(product_id)
            .one(db)
            .await
            .map_err(|e| e.to_string())?
    {
        let mut am: product::ActiveModel = existing.into();
        am.updated_at = Set(Some(Utc::now()));
        am.name = Set(name);
        am.variables = Set(variables);
        am.base_price_formula = Set(formula.clone());
        am.sales_price_formula = Set(formula);
        let saved = am.update(db).await.map_err(|e| e.to_string())?;
        return Ok(saved.id);
    }
    insert_product(db, name, variables, formula).await
}

/// Create or update the goods product that stores a component's schema and cost formula.
pub async fn save_component_product<C: ConnectionTrait>(
    db: &C,
    product_id: i64,
    name: &str,
    variables: &serde_json::Value,
    cost_formula: &str,
) -> Result<i64, String> {
    write_product(
        db,
        product_id,
        display_name(name, "Component"),
        schema_text(variables),
        formula_text(cost_formula, Decimal::ZERO),
    )
    .await
}

/// Product linked to this component, creating one when the link is missing.
pub async fn ensure_component_product<C: ConnectionTrait>(
    db: &C,
    component: &crate::entities::component::Model,
) -> Result<i64, String> {
    if component.product_id > 0 {
        let exists = product::Entity::find_by_id(component.product_id)
            .one(db)
            .await
            .map_err(|e| e.to_string())?
            .is_some();
        if exists {
            return Ok(component.product_id);
        }
    }
    let id = save_component_product(
        db,
        0,
        &component.name,
        &component.variables,
        &component.cost_formula,
    )
    .await?;
    let mut am: crate::entities::component::ActiveModel = component.clone().into();
    am.product_id = Set(id);
    am.update(db).await.map_err(|e| e.to_string())?;
    Ok(id)
}

pub async fn component_product_id<C: ConnectionTrait>(
    db: &C,
    component_id: i64,
) -> Result<i64, String> {
    let component = crate::entities::component::Entity::find_by_id(component_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Component #{component_id} not found"))?;
    ensure_component_product(db, &component).await
}

/// Service linked to this machine, creating one when the link is missing.
pub async fn machine_product_id<C: ConnectionTrait>(
    db: &C,
    machine_id: i64,
) -> Result<i64, String> {
    kds_plugin_machinery_schedule::machine_product::machine_product_id(db, machine_id).await
}

/// Line text wins. A blank line uses the linked service's remarks.
pub fn remarks_with_product_default(line_remarks: &str, product_remarks: Option<&str>) -> String {
    let line = line_remarks.trim();
    if !line.is_empty() {
        return line.to_string();
    }
    product_remarks.unwrap_or("").trim().to_string()
}

pub async fn product_remarks_by_id<C: ConnectionTrait>(
    db: &C,
    ids: &[i64],
) -> HashMap<i64, String> {
    let ids: Vec<i64> = ids.iter().copied().filter(|id| *id > 0).collect();
    if ids.is_empty() {
        return HashMap::new();
    }
    kds_plugin_machinery_schedule::machine_product::service_remarks_by_id(db, &ids).await
}

#[cfg(test)]
mod tests {
    use super::remarks_with_product_default;

    #[test]
    fn blank_line_remarks_use_the_product() {
        assert_eq!(
            remarks_with_product_default("  ", Some("  From the product  ")),
            "From the product"
        );
        assert_eq!(
            remarks_with_product_default(" On the line ", Some("From the product")),
            "On the line"
        );
        assert_eq!(remarks_with_product_default("", None), "");
    }
}
