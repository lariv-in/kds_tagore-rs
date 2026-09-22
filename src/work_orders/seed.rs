//! Seeding of example components and machines with Rune cost formulas.

use chrono::Utc;
use sea_orm::{ActiveModelTrait, ConnectionTrait, EntityTrait, PaginatorTrait, Set};

use super::entities::component;
use crate::formula::{VariableType, schema_to_json};
use crate::machinery_schedule::entities::machine;

fn ms_flat_schema() -> serde_json::Value {
    let mut map = std::collections::HashMap::new();
    map.insert("length".into(), VariableType::Length);
    map.insert("qty".into(), VariableType::Quantity);
    schema_to_json(&map)
}

fn ss_rod_schema() -> serde_json::Value {
    let mut map = std::collections::HashMap::new();
    map.insert("length".into(), VariableType::Length);
    map.insert("qty".into(), VariableType::Quantity);
    schema_to_json(&map)
}

fn machine_schema() -> serde_json::Value {
    let mut map = std::collections::HashMap::new();
    map.insert("duration".into(), VariableType::Duration);
    schema_to_json(&map)
}

/// Idempotent: inserts example components and machines if those tables are empty.
pub async fn ensure_standard_seeds<C: ConnectionTrait>(db: &C) -> Result<(), sea_orm::DbErr> {
    ensure_standard_machines(db).await?;
    ensure_standard_components(db).await
}

/// Kept for historical migration call sites; seeding now happens in later migrations.
pub async fn ensure_standard_bases<C: ConnectionTrait>(_db: &C) -> Result<(), sea_orm::DbErr> {
    Ok(())
}

pub async fn ensure_standard_machines<C: ConnectionTrait>(db: &C) -> Result<(), sea_orm::DbErr> {
    let machine_count = machine::Entity::find().count(db).await.unwrap_or(0);
    if machine_count == 0 {
        let now = Utc::now();
        let schema = machine_schema();
        let machines = [
            ("CNC Turning Center (Lathe)", "duration / 3600 * 950"),
            ("VMC 3-Axis Milling Center", "duration / 3600 * 1250"),
            ("Surface Grinding Machine", "duration / 3600 * 550"),
            ("Heavy-Duty Band Saw", "duration / 3600 * 350"),
        ];
        for (name, formula) in machines {
            let active = machine::ActiveModel {
                id: Default::default(),
                created_at: Set(Some(now)),
                updated_at: Set(Some(now)),
                name: Set(name.to_string()),
                cost_formula: Set(formula.to_string()),
                variables: Set(schema.clone()),
            };
            active.insert(db).await?;
        }
    }
    Ok(())
}

pub async fn ensure_standard_components<C: ConnectionTrait>(db: &C) -> Result<(), sea_orm::DbErr> {
    let comp_count = component::Entity::find().count(db).await.unwrap_or(0);
    if comp_count == 0 {
        let now = Utc::now();
        let bar = component::ActiveModel {
            id: Default::default(),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            name: Set("MS Flat Bar 2.5x3.5mm".to_string()),
            variables: Set(ms_flat_schema()),
            // volume mm³ / 1e9 * density 7850 * rate 85 * qty; 2.5 * 3.5 * length mm
            weight_formula: Set(
                "length * decimal(\"2.5\") * decimal(\"3.5\") / 1000000000 * 7850 * qty".into(),
            ),
            cost_formula: Set(
                "length * decimal(\"2.5\") * decimal(\"3.5\") / 1000000000 * 7850 * 85 * qty"
                    .into(),
            ),
        };
        bar.insert(db).await?;

        let rod = component::ActiveModel {
            id: Default::default(),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            name: Set("SS 304 Round Rod Ø20mm".to_string()),
            variables: Set(ss_rod_schema()),
            weight_formula: Set("length * 314 * qty / 1000000000 * 7930".into()),
            cost_formula: Set("length * 314 * qty / 1000000000 * 7930 * 380".into()),
        };
        rod.insert(db).await?;
    }
    Ok(())
}
