//! Seeding of built-in standard shapes, common manufacturing stock materials, and machines.

use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter, Set,
};

use super::entities::{component, machine, material, material_rate, shape};
use super::geometry::PrimitiveKind;

/// Ensures standard shapes, materials, rates, and machines exist in the database.
/// Idempotent: only inserts if tables are empty.
pub async fn ensure_standard_seeds<C: ConnectionTrait>(db: &C) -> Result<(), sea_orm::DbErr> {
    // 1. Seed standard shapes
    let shape_count = shape::Entity::find().count(db).await.unwrap_or(0);
    if shape_count == 0 {
        let now = Utc::now();
        for kind in PrimitiveKind::all() {
            let var_names: Vec<String> = kind
                .required_variables()
                .iter()
                .map(|s| s.to_string())
                .collect();
            let active = shape::ActiveModel {
                id: Default::default(),
                created_at: Set(Some(now)),
                updated_at: Set(Some(now)),
                name: Set(kind.display_name().to_string()),
                openscad_code: Set(kind.default_openscad_code().to_string()),
                variable_names: Set(serde_json::to_value(&var_names).unwrap_or_default()),
            };
            let _ = active.insert(db).await?;
        }
    }

    // 2. Seed standard materials and their rates
    let mat_count = material::Entity::find().count(db).await.unwrap_or(0);
    if mat_count == 0 {
        let now = Utc::now();
        let materials = [
            ("Mild Steel (MS)", 7850.0, Decimal::new(8500, 2)),          // ₹85.00/kg
            ("Stainless Steel 304 (SS 304)", 7930.0, Decimal::new(38000, 2)), // ₹380.00/kg
            ("Aluminum 6061-T6", 2700.0, Decimal::new(29000, 2)),       // ₹290.00/kg
            ("Brass (CuZn39Pb3)", 8500.0, Decimal::new(52000, 2)),       // ₹520.00/kg
        ];

        for (name, density, rate) in materials {
            let active_mat = material::ActiveModel {
                id: Default::default(),
                created_at: Set(Some(now)),
                updated_at: Set(Some(now)),
                name: Set(name.to_string()),
                density: Set(density),
            };
            if let Ok(inserted) = active_mat.insert(db).await {
                let active_rate = material_rate::ActiveModel {
                    id: Default::default(),
                    created_at: Set(Some(now)),
                    updated_at: Set(Some(now)),
                    material_id: Set(inserted.id),
                    rate_decimal: Set(rate),
                    datetime: Set(now),
                };
                let _ = active_rate.insert(db).await;
            }
        }
    }

    // 3. Seed standard machines
    let machine_count = machine::Entity::find().count(db).await.unwrap_or(0);
    if machine_count == 0 {
        let now = Utc::now();
        let machines = [
            ("CNC Turning Center (Lathe)", Decimal::new(95000, 2)), // ₹950.00/hr
            ("VMC 3-Axis Milling Center", Decimal::new(125000, 2)), // ₹1250.00/hr
            ("Surface Grinding Machine", Decimal::new(55000, 2)),   // ₹550.00/hr
            ("Heavy-Duty Band Saw", Decimal::new(35000, 2)),        // ₹350.00/hr
        ];

        for (name, rate) in machines {
            let active_machine = machine::ActiveModel {
                id: Default::default(),
                created_at: Set(Some(now)),
                updated_at: Set(Some(now)),
                name: Set(name.to_string()),
                rate_decimal: Set(rate),
            };
            let _ = active_machine.insert(db).await;
        }
    }

    // 4. Seed standard components with fixed stock dimensions in mm
    let comp_count = component::Entity::find().count(db).await.unwrap_or(0);
    if comp_count == 0 {
        let now = Utc::now();
        let ms_mat = material::Entity::find()
            .filter(material::Column::Name.contains("Mild Steel"))
            .one(db)
            .await?;
        let ss_mat = material::Entity::find()
            .filter(material::Column::Name.contains("Stainless Steel"))
            .one(db)
            .await?;
        let box_shape = shape::Entity::find()
            .filter(shape::Column::Name.contains("Box"))
            .one(db)
            .await?;
        let cyl_shape = shape::Entity::find()
            .filter(shape::Column::Name.contains("Cylinder"))
            .one(db)
            .await?;

        if let (Some(ms), Some(bx)) = (ms_mat.as_ref(), box_shape.as_ref()) {
            let mut fixed = serde_json::Map::new();
            fixed.insert("width".into(), serde_json::json!(2.5));
            fixed.insert("thickness".into(), serde_json::json!(3.5));

            let bar_comp = component::ActiveModel {
                id: Default::default(),
                created_at: Set(Some(now)),
                updated_at: Set(Some(now)),
                name: Set("MS Flat Bar 2.5x3.5mm".to_string()),
                shape_id: Set(bx.id),
                material_id: Set(ms.id),
                fixed_variables: Set(serde_json::Value::Object(fixed)),
            };
            let _ = bar_comp.insert(db).await;
        }

        if let (Some(ss), Some(cyl)) = (ss_mat.as_ref(), cyl_shape.as_ref()) {
            let mut fixed = serde_json::Map::new();
            fixed.insert("diameter".into(), serde_json::json!(20.0));

            let rod_comp = component::ActiveModel {
                id: Default::default(),
                created_at: Set(Some(now)),
                updated_at: Set(Some(now)),
                name: Set("SS 304 Round Rod Ø20mm".to_string()),
                shape_id: Set(cyl.id),
                material_id: Set(ss.id),
                fixed_variables: Set(serde_json::Value::Object(fixed)),
            };
            let _ = rod_comp.insert(db).await;
        }
    }

    Ok(())
}
