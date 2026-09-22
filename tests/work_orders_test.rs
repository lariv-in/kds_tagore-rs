use kds_tagore_rs::formula::{
    VariableType, VariableValue, VariableValues, eval_formula, standard_sample_values,
    validate_formula,
};
use kds_tagore_rs::work_orders::{
    entities::{component, quotation, quotation_machine_line, quotation_material_line},
    seed,
};
use rust_decimal::Decimal;
use sea_orm::{EntityTrait, PaginatorTrait};
use std::collections::HashMap;

async fn test_db() -> sea_orm::DatabaseConnection {
    use sea_orm_migration::MigratorTrait;
    let db = sea_orm::Database::connect("sqlite::memory:")
        .await
        .expect("sqlite db");
    struct CombinedMigrator;
    impl MigratorTrait for CombinedMigrator {
        fn migrations() -> Vec<Box<dyn sea_orm_migration::MigrationTrait>> {
            let mut all = kds_tagore_rs::machinery_schedule::migrations::Migrator::migrations();
            all.extend(kds_tagore_rs::work_orders::migrations::Migrator::migrations());
            all
        }
    }
    CombinedMigrator::up(&db, None)
        .await
        .expect("combined migrator up");
    db
}

fn gst18_levied_tax() -> lariv_rs::plugins::finance_taxes::entities::tax::Model {
    lariv_rs::plugins::finance_taxes::entities::tax::Model {
        id: 1,
        created_at: None,
        updated_at: None,
        name: "GST".into(),
        percentage: Decimal::from(18),
        tax_type: lariv_rs::plugins::finance_taxes::entities::TaxKind::Levied,
        account_id: None,
    }
}

fn sample_component() -> component::Model {
    let mut schema = HashMap::new();
    schema.insert("length".into(), VariableType::Length);
    schema.insert("qty".into(), VariableType::Quantity);
    component::Model {
        id: 1,
        created_at: None,
        updated_at: None,
        name: "Bar".into(),
        cost_formula: "length * qty * 2".into(),
        weight_formula: "length * qty / 1000".into(),
        variables: kds_tagore_rs::formula::schema_to_json(&schema),
    }
}

#[test]
fn test_component_get_cost_and_weight_decimal() {
    let c = sample_component();
    let mut vars = VariableValues::new();
    vars.insert("length".into(), VariableValue::Length(Decimal::from(10)));
    vars.insert("qty".into(), VariableValue::Quantity(3));
    assert_eq!(c.get_cost(&vars).unwrap(), Decimal::from(60));
    assert_eq!(
        c.get_weight(&vars).unwrap(),
        Decimal::from_str_exact("0.03").unwrap()
    );
}

#[test]
fn test_validate_formulas_standard_samples() {
    let c = sample_component();
    c.validate_formulas().expect("sample values should succeed");
    let mut bad = c.clone();
    bad.cost_formula = "not_defined".into();
    assert!(bad.validate_formulas().is_err());
}

#[test]
fn test_machine_get_cost_duration_seconds() {
    use kds_tagore_rs::machinery_schedule::entities::machine;
    let mut schema = HashMap::new();
    schema.insert("duration".into(), VariableType::Duration);
    let m = machine::Model {
        id: 1,
        created_at: None,
        updated_at: None,
        name: "Lathe".into(),
        cost_formula: "duration / 3600 * 950".into(),
        variables: kds_tagore_rs::formula::schema_to_json(&schema),
    };
    let mut vars = VariableValues::new();
    vars.insert(
        "duration".into(),
        VariableValue::DurationNanos(3_600_000_000_000),
    );
    assert_eq!(m.get_cost(&vars).unwrap(), Decimal::from(950));
    m.validate_formulas().unwrap();
}

#[test]
fn test_invoice_line_totals_and_taxes() {
    let pretax = Decimal::from_str_exact("1000").unwrap();
    let mat = quotation_material_line::Model {
        id: 1,
        created_at: None,
        updated_at: None,
        invoice_id: 10,
        component_id: 1,
        variables: serde_json::json!({"qty": 1}),
        final_cost: pretax,
        extra_data: serde_json::json!({}),
    };
    let mach = quotation_machine_line::Model {
        id: 1,
        created_at: None,
        updated_at: None,
        invoice_id: 10,
        machine_id: Some(1),
        name: "Milling".into(),
        variables: serde_json::json!({}),
        final_cost: pretax,
    };
    let inv = quotation::Model {
        id: 10,
        created_at: None,
        updated_at: None,
        date: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
        customer_id: 1,
        invoice_number: "Q-1".into(),
    };
    assert_eq!(
        inv.grand_total(&[mat.clone()], &[mach.clone()]),
        pretax * Decimal::from(2)
    );
    let tax = gst18_levied_tax();
    assert_eq!(
        mat.taxed_total(&[tax.clone()]),
        Decimal::from_str_exact("1180").unwrap()
    );
    assert_eq!(
        mach.taxed_total(&[tax]),
        Decimal::from_str_exact("1180").unwrap()
    );
}

#[test]
fn test_eval_formula_no_f64() {
    let mut schema = HashMap::new();
    schema.insert("duration".into(), VariableType::Duration);
    let mut values = VariableValues::new();
    values.insert(
        "duration".into(),
        VariableValue::DurationNanos(7_200_000_000_000),
    );
    let out = eval_formula(&schema, "duration / 3600 * 100", &values).unwrap();
    assert_eq!(out, Decimal::from(200));
    let samples = standard_sample_values(&schema);
    validate_formula(&schema, "duration * 1").unwrap();
    assert!(samples.contains_key("duration"));
}

#[tokio::test]
async fn test_work_orders_migration_and_seed() {
    let db = test_db().await;
    seed::ensure_standard_seeds(&db).await.expect("seed");
    let n = component::Entity::find()
        .count(&db)
        .await
        .expect("count components");
    assert!(n >= 1, "standard components should be seeded");
}
