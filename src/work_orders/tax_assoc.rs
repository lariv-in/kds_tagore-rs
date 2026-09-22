//! M2M tax association helpers and taxed-amount math for work-order lines.

use std::collections::HashMap;

use lariv_rs::plugins::finance_invoices::logic::tax_calculations::invoice_line_amount_breakdown;
use lariv_rs::plugins::finance_taxes::{
    entities::tax,
    scope::{load_all_taxes, load_taxes_by_ids, tax_label},
};
use rust_decimal::Decimal;
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, Statement, Value};

const DRAFT_MATERIAL: &str = "draft_work_order_material_line_taxes";
const DRAFT_MACHINE: &str = "draft_work_order_machine_line_taxes";
const QUOTATION_MATERIAL: &str = "kds_quotation_material_line_taxes";
const QUOTATION_MACHINE: &str = "kds_quotation_machine_line_taxes";
const WORK_ORDER_MATERIAL: &str = "kds_work_order_material_line_taxes";
const WORK_ORDER_MACHINE: &str = "kds_work_order_machine_line_taxes";
const PREFS_MATERIAL: &str = "work_orders_preferences_default_material_taxes";
const PREFS_MACHINE: &str = "work_orders_preferences_default_machine_taxes";
const PREFS_ID: i64 = 1;

fn ph(backend: DatabaseBackend, n: usize) -> String {
    match backend {
        DatabaseBackend::Postgres => format!("${n}"),
        _ => "?".to_string(),
    }
}

async fn set_owner_taxes<C: ConnectionTrait>(
    db: &C,
    table: &str,
    owner_col: &str,
    owner_id: i64,
    tax_ids: &[i64],
) -> Result<(), sea_orm::DbErr> {
    let backend = db.get_database_backend();
    db.execute_raw(Statement::from_sql_and_values(
        backend,
        format!("DELETE FROM {table} WHERE {owner_col} = {}", ph(backend, 1)),
        [owner_id.into()],
    ))
    .await?;
    let mut seen = std::collections::HashSet::new();
    for tax_id in tax_ids {
        if *tax_id <= 0 || !seen.insert(*tax_id) {
            continue;
        }
        db.execute_raw(Statement::from_sql_and_values(
            backend,
            format!(
                "INSERT INTO {table} ({owner_col}, tax_id) VALUES ({}, {})",
                ph(backend, 1),
                ph(backend, 2)
            ),
            [owner_id.into(), (*tax_id).into()],
        ))
        .await?;
    }
    Ok(())
}

async fn set_line_taxes<C: ConnectionTrait>(
    db: &C,
    table: &str,
    line_id: i64,
    tax_ids: &[i64],
) -> Result<(), sea_orm::DbErr> {
    set_owner_taxes(db, table, "line_id", line_id, tax_ids).await
}

async fn load_owner_tax_ids<C: ConnectionTrait>(
    db: &C,
    table: &str,
    owner_col: &str,
    owner_id: i64,
) -> Result<Vec<i64>, sea_orm::DbErr> {
    let backend = db.get_database_backend();
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(
            backend,
            format!(
                "SELECT tax_id FROM {table} WHERE {owner_col} = {}",
                ph(backend, 1)
            ),
            [owner_id.into()],
        ))
        .await?;
    Ok(rows
        .into_iter()
        .filter_map(|r| r.try_get::<i64>("", "tax_id").ok())
        .collect())
}

async fn load_line_tax_ids<C: ConnectionTrait>(
    db: &C,
    table: &str,
    line_id: i64,
) -> Result<Vec<i64>, sea_orm::DbErr> {
    load_owner_tax_ids(db, table, "line_id", line_id).await
}

async fn load_line_tax_ids_map<C: ConnectionTrait>(
    db: &C,
    table: &str,
    line_ids: &[i64],
) -> Result<HashMap<i64, Vec<i64>>, sea_orm::DbErr> {
    let mut map: HashMap<i64, Vec<i64>> = HashMap::new();
    if line_ids.is_empty() {
        return Ok(map);
    }
    let backend = db.get_database_backend();
    let placeholders: Vec<String> = line_ids
        .iter()
        .enumerate()
        .map(|(i, _)| ph(backend, i + 1))
        .collect();
    let sql = format!(
        "SELECT line_id, tax_id FROM {table} WHERE line_id IN ({})",
        placeholders.join(", ")
    );
    let values: Vec<Value> = line_ids.iter().copied().map(Into::into).collect();
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(backend, sql, values))
        .await?;
    for row in rows {
        let Ok(line_id) = row.try_get::<i64>("", "line_id") else {
            continue;
        };
        let Ok(tax_id) = row.try_get::<i64>("", "tax_id") else {
            continue;
        };
        map.entry(line_id).or_default().push(tax_id);
    }
    Ok(map)
}

pub async fn set_draft_material_line_taxes<C: ConnectionTrait>(
    db: &C,
    line_id: i64,
    tax_ids: &[i64],
) -> Result<(), sea_orm::DbErr> {
    set_line_taxes(db, DRAFT_MATERIAL, line_id, tax_ids).await
}

pub async fn load_draft_material_line_tax_ids<C: ConnectionTrait>(
    db: &C,
    line_id: i64,
) -> Result<Vec<i64>, sea_orm::DbErr> {
    load_line_tax_ids(db, DRAFT_MATERIAL, line_id).await
}

pub async fn load_draft_material_line_tax_ids_map<C: ConnectionTrait>(
    db: &C,
    line_ids: &[i64],
) -> Result<HashMap<i64, Vec<i64>>, sea_orm::DbErr> {
    load_line_tax_ids_map(db, DRAFT_MATERIAL, line_ids).await
}

pub async fn set_draft_machine_line_taxes<C: ConnectionTrait>(
    db: &C,
    line_id: i64,
    tax_ids: &[i64],
) -> Result<(), sea_orm::DbErr> {
    set_line_taxes(db, DRAFT_MACHINE, line_id, tax_ids).await
}

pub async fn load_draft_machine_line_tax_ids<C: ConnectionTrait>(
    db: &C,
    line_id: i64,
) -> Result<Vec<i64>, sea_orm::DbErr> {
    load_line_tax_ids(db, DRAFT_MACHINE, line_id).await
}

pub async fn load_draft_machine_line_tax_ids_map<C: ConnectionTrait>(
    db: &C,
    line_ids: &[i64],
) -> Result<HashMap<i64, Vec<i64>>, sea_orm::DbErr> {
    load_line_tax_ids_map(db, DRAFT_MACHINE, line_ids).await
}

pub async fn set_quotation_material_line_taxes<C: ConnectionTrait>(
    db: &C,
    line_id: i64,
    tax_ids: &[i64],
) -> Result<(), sea_orm::DbErr> {
    set_line_taxes(db, QUOTATION_MATERIAL, line_id, tax_ids).await
}

pub async fn load_quotation_material_line_tax_ids<C: ConnectionTrait>(
    db: &C,
    line_id: i64,
) -> Result<Vec<i64>, sea_orm::DbErr> {
    load_line_tax_ids(db, QUOTATION_MATERIAL, line_id).await
}

pub async fn load_quotation_material_line_tax_ids_map<C: ConnectionTrait>(
    db: &C,
    line_ids: &[i64],
) -> Result<HashMap<i64, Vec<i64>>, sea_orm::DbErr> {
    load_line_tax_ids_map(db, QUOTATION_MATERIAL, line_ids).await
}

pub async fn set_quotation_machine_line_taxes<C: ConnectionTrait>(
    db: &C,
    line_id: i64,
    tax_ids: &[i64],
) -> Result<(), sea_orm::DbErr> {
    set_line_taxes(db, QUOTATION_MACHINE, line_id, tax_ids).await
}

pub async fn load_quotation_machine_line_tax_ids<C: ConnectionTrait>(
    db: &C,
    line_id: i64,
) -> Result<Vec<i64>, sea_orm::DbErr> {
    load_line_tax_ids(db, QUOTATION_MACHINE, line_id).await
}

pub async fn load_quotation_machine_line_tax_ids_map<C: ConnectionTrait>(
    db: &C,
    line_ids: &[i64],
) -> Result<HashMap<i64, Vec<i64>>, sea_orm::DbErr> {
    load_line_tax_ids_map(db, QUOTATION_MACHINE, line_ids).await
}

pub async fn set_work_order_material_line_taxes<C: ConnectionTrait>(
    db: &C,
    line_id: i64,
    tax_ids: &[i64],
) -> Result<(), sea_orm::DbErr> {
    set_line_taxes(db, WORK_ORDER_MATERIAL, line_id, tax_ids).await
}

pub async fn load_work_order_material_line_tax_ids<C: ConnectionTrait>(
    db: &C,
    line_id: i64,
) -> Result<Vec<i64>, sea_orm::DbErr> {
    load_line_tax_ids(db, WORK_ORDER_MATERIAL, line_id).await
}

pub async fn load_work_order_material_line_tax_ids_map<C: ConnectionTrait>(
    db: &C,
    line_ids: &[i64],
) -> Result<HashMap<i64, Vec<i64>>, sea_orm::DbErr> {
    load_line_tax_ids_map(db, WORK_ORDER_MATERIAL, line_ids).await
}

pub async fn set_work_order_machine_line_taxes<C: ConnectionTrait>(
    db: &C,
    line_id: i64,
    tax_ids: &[i64],
) -> Result<(), sea_orm::DbErr> {
    set_line_taxes(db, WORK_ORDER_MACHINE, line_id, tax_ids).await
}

pub async fn load_work_order_machine_line_tax_ids<C: ConnectionTrait>(
    db: &C,
    line_id: i64,
) -> Result<Vec<i64>, sea_orm::DbErr> {
    load_line_tax_ids(db, WORK_ORDER_MACHINE, line_id).await
}

pub async fn load_work_order_machine_line_tax_ids_map<C: ConnectionTrait>(
    db: &C,
    line_ids: &[i64],
) -> Result<HashMap<i64, Vec<i64>>, sea_orm::DbErr> {
    load_line_tax_ids_map(db, WORK_ORDER_MACHINE, line_ids).await
}

pub async fn set_default_material_taxes<C: ConnectionTrait>(
    db: &C,
    tax_ids: &[i64],
) -> Result<(), sea_orm::DbErr> {
    set_owner_taxes(db, PREFS_MATERIAL, "prefs_id", PREFS_ID, tax_ids).await
}

pub async fn load_default_material_tax_ids<C: ConnectionTrait>(
    db: &C,
) -> Result<Vec<i64>, sea_orm::DbErr> {
    load_owner_tax_ids(db, PREFS_MATERIAL, "prefs_id", PREFS_ID).await
}

pub async fn set_default_machine_taxes<C: ConnectionTrait>(
    db: &C,
    tax_ids: &[i64],
) -> Result<(), sea_orm::DbErr> {
    set_owner_taxes(db, PREFS_MACHINE, "prefs_id", PREFS_ID, tax_ids).await
}

pub async fn load_default_machine_tax_ids<C: ConnectionTrait>(
    db: &C,
) -> Result<Vec<i64>, sea_orm::DbErr> {
    load_owner_tax_ids(db, PREFS_MACHINE, "prefs_id", PREFS_ID).await
}

pub fn normalize_tax_ids(ids: &[i64]) -> Vec<i64> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for id in ids {
        if *id > 0 && seen.insert(*id) {
            out.push(*id);
        }
    }
    out
}

/// Apply levied (+) and withholding (−) taxes to a pre-tax amount.
pub fn taxed_amount(untaxed: Decimal, taxes: &[tax::Model]) -> Decimal {
    let (_, _, _, net) = invoice_line_amount_breakdown(Decimal::ONE, untaxed, taxes);
    net
}

pub fn tax_labels_display(taxes: &[tax::Model]) -> String {
    if taxes.is_empty() {
        return "—".into();
    }
    taxes.iter().map(tax_label).collect::<Vec<_>>().join(", ")
}

pub async fn resolve_taxes_by_line_id(
    db: &DatabaseConnection,
    ids_by_line: &HashMap<i64, Vec<i64>>,
) -> HashMap<i64, Vec<tax::Model>> {
    let mut all_ids: Vec<i64> = ids_by_line.values().flatten().copied().collect();
    all_ids.sort_unstable();
    all_ids.dedup();
    let loaded = load_taxes_by_ids(db, &all_ids).await.unwrap_or_default();
    let by_id: HashMap<i64, tax::Model> = loaded.into_iter().map(|t| (t.id, t)).collect();
    let mut out = HashMap::new();
    for (line_id, tax_ids) in ids_by_line {
        let taxes: Vec<tax::Model> = tax_ids
            .iter()
            .filter_map(|id| by_id.get(id).cloned())
            .collect();
        out.insert(*line_id, taxes);
    }
    out
}

pub fn taxes_for_line<'a>(
    taxes_by_line: &'a HashMap<i64, Vec<tax::Model>>,
    line_id: i64,
) -> &'a [tax::Model] {
    taxes_by_line
        .get(&line_id)
        .map(|v| v.as_slice())
        .unwrap_or(&[])
}

pub async fn taxes_catalog_json(db: &DatabaseConnection) -> String {
    let taxes = load_all_taxes(db).await.unwrap_or_default();
    let mut tax_pct_by_id = serde_json::Map::new();
    let mut tax_kind_by_id = serde_json::Map::new();
    let mut all_taxes = Vec::new();
    for t in &taxes {
        tax_pct_by_id.insert(
            t.id.to_string(),
            serde_json::Value::String(t.percentage.normalize().to_string()),
        );
        tax_kind_by_id.insert(
            t.id.to_string(),
            serde_json::Value::String(t.tax_type.as_str().to_string()),
        );
        all_taxes.push(serde_json::json!({
            "id": t.id,
            "name": tax_label(t),
            "tax_kind": t.tax_type.as_str(),
        }));
    }
    let default_material_ids = load_default_material_tax_ids(db).await.unwrap_or_default();
    let default_machine_ids = load_default_machine_tax_ids(db).await.unwrap_or_default();
    serde_json::json!({
        "tax_pct_by_id": tax_pct_by_id,
        "tax_kind_by_id": tax_kind_by_id,
        "all_taxes": all_taxes,
        "default_material_taxes": tax_entries(&taxes, &default_material_ids),
        "default_machine_taxes": tax_entries(&taxes, &default_machine_ids),
    })
    .to_string()
}

fn tax_entries(taxes: &[tax::Model], ids: &[i64]) -> Vec<serde_json::Value> {
    ids.iter()
        .filter_map(|id| {
            taxes.iter().find(|t| t.id == *id).map(|t| {
                serde_json::json!({
                    "id": t.id,
                    "name": tax_label(t),
                })
            })
        })
        .collect()
}

pub fn parse_tax_ids_json(value: Option<&serde_json::Value>) -> Vec<i64> {
    let Some(v) = value else {
        return Vec::new();
    };
    match v {
        serde_json::Value::Array(arr) => arr
            .iter()
            .filter_map(|x| match x {
                serde_json::Value::Number(n) => n.as_i64(),
                serde_json::Value::String(s) => s.trim().parse().ok(),
                _ => None,
            })
            .filter(|id| *id > 0)
            .collect(),
        serde_json::Value::String(s) => s
            .split(',')
            .filter_map(|p| p.trim().parse().ok())
            .filter(|id: &i64| *id > 0)
            .collect(),
        serde_json::Value::Number(n) => n.as_i64().filter(|id| *id > 0).into_iter().collect(),
        _ => Vec::new(),
    }
}
