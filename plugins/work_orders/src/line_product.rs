//! Products backing quotation and work-order lines.
//!
//! A component owns one goods product. Each machine line owns a services product.
//! Variable schemas and price formulas live on that product; the line keeps the
//! values entered for that job.

use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ConnectionTrait, EntityTrait};
use uuid::Uuid;

use lariv_plugin_finance_products::entities::product::{self, ProductType};

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
    product_type: ProductType,
    variables: String,
    formula: String,
) -> Result<i64, String> {
    let now = Utc::now();
    let saved = product::ActiveModel {
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        product_type: Set(product_type),
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
    product_type: ProductType,
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
        am.product_type = Set(product_type);
        am.variables = Set(variables);
        am.base_price_formula = Set(formula.clone());
        am.sales_price_formula = Set(formula);
        let saved = am.update(db).await.map_err(|e| e.to_string())?;
        return Ok(saved.id);
    }
    insert_product(db, name, product_type, variables, formula).await
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
        ProductType::Goods,
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

/// Create or update the services product for a machine line.
///
/// The product takes the machine's variable schema and cost formula. `fallback_cost`
/// is the sales formula when the machine has no formula.
pub async fn save_machine_line_product<C: ConnectionTrait>(
    db: &C,
    product_id: i64,
    machine_id: Option<i64>,
    name: &str,
    fallback_cost: Decimal,
) -> Result<i64, String> {
    let mach = match machine_id.filter(|id| *id > 0) {
        Some(id) => kds_plugin_machinery_schedule::entities::machine::Entity::find_by_id(id)
            .one(db)
            .await
            .map_err(|e| e.to_string())?,
        None => None,
    };
    let display = match name.trim() {
        "" => mach
            .as_ref()
            .map(|m| m.name.clone())
            .filter(|n| !n.trim().is_empty())
            .unwrap_or_else(|| "Machine line".into()),
        other => other.to_string(),
    };
    let (schema, formula) = match &mach {
        Some(m) => (
            schema_text(&m.variables),
            formula_text(&m.cost_formula, fallback_cost),
        ),
        None => ("{}".into(), formula_text("", fallback_cost)),
    };
    write_product(
        db,
        product_id,
        display,
        ProductType::Services,
        schema,
        formula,
    )
    .await
}
