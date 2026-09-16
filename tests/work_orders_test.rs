use std::collections::HashMap;
use chrono::Utc;
use rust_decimal::Decimal;
use kds_tagore_rs::machinery_schedule::duration::JobDuration;
use kds_tagore_rs::work_orders::{
    entities::{
        component, machine, material, proforma_invoice_machine_line,
        proforma_invoice_material_line, shape,
    },
    geometry::{
        calculate_openscad_volume, calculate_stl_volume, PrimitiveKind,
    },
};

#[test]
fn test_stl_volume_with_stl_io() {
    // 1x1x1 cube in ASCII STL
    let cube_stl = r#"solid cube
facet normal 0 0 -1
  outer loop
    vertex 0 0 0
    vertex 1 0 0
    vertex 1 1 0
  endloop
endfacet
facet normal 0 0 -1
  outer loop
    vertex 0 0 0
    vertex 1 1 0
    vertex 0 1 0
  endloop
endfacet
facet normal 0 0 1
  outer loop
    vertex 0 0 1
    vertex 1 1 1
    vertex 1 0 1
  endloop
endfacet
facet normal 0 0 1
  outer loop
    vertex 0 0 1
    vertex 0 1 1
    vertex 1 1 1
  endloop
endfacet
facet normal 0 -1 0
  outer loop
    vertex 0 0 0
    vertex 1 0 1
    vertex 1 0 0
  endloop
endfacet
facet normal 0 -1 0
  outer loop
    vertex 0 0 0
    vertex 0 0 1
    vertex 1 0 1
  endloop
endfacet
facet normal 0 1 0
  outer loop
    vertex 0 1 0
    vertex 1 1 0
    vertex 1 1 1
  endloop
endfacet
facet normal 0 1 0
  outer loop
    vertex 0 1 0
    vertex 1 1 1
    vertex 0 1 1
  endloop
endfacet
facet normal -1 0 0
  outer loop
    vertex 0 0 0
    vertex 0 1 0
    vertex 0 1 1
  endloop
endfacet
facet normal -1 0 0
  outer loop
    vertex 0 0 0
    vertex 0 1 1
    vertex 0 0 1
  endloop
endfacet
facet normal 1 0 0
  outer loop
    vertex 1 0 0
    vertex 1 1 1
    vertex 1 1 0
  endloop
endfacet
facet normal 1 0 0
  outer loop
    vertex 1 0 0
    vertex 1 0 1
    vertex 1 1 1
  endloop
endfacet
endsolid cube"#;

    let volume = calculate_stl_volume(cube_stl.as_bytes()).expect("calculate_stl_volume");
    assert!((volume - 1.0).abs() < 1e-5, "Expected 1.0 m³, got {}", volume);
}

#[tokio::test]
async fn test_openscad_execution_and_volume() {
    let code = "cube([x, y, z]);";
    let mut vars = HashMap::new();
    vars.insert("x".into(), 100.0); // 100mm
    vars.insert("y".into(), 200.0); // 200mm
    vars.insert("z".into(), 300.0); // 300mm

    let volume = calculate_openscad_volume(code, &vars).await.expect("calculate_openscad_volume");
    // 100 * 200 * 300 mm3 = 6,000,000 mm3 = 0.006 m3
    assert!((volume - 0.006).abs() < 1e-6, "Expected volume 0.006 m³, got {}", volume);
}

#[test]
fn test_shape_get_volume_method() {
    let s = shape::Model {
        id: 1,
        created_at: Some(Utc::now()),
        updated_at: Some(Utc::now()),
        name: "Test Block".into(),
        openscad_code: "cube([length, width, thickness]);".into(),
        variable_names: serde_json::json!(["length", "width", "thickness"]),
    };

    let mut vars = HashMap::new();
    vars.insert("length".into(), 150.0); // 150mm
    vars.insert("width".into(), 200.0);  // 200mm
    vars.insert("thickness".into(), 300.0); // 300mm

    let vol = s.get_volume(vars);
    // 150 * 200 * 300 mm3 = 9,000,000 mm3 = 0.009 m3
    assert!((vol - 0.009).abs() < 1e-6, "Expected volume 0.009 m³, got {}", vol);
}

#[test]
fn test_component_get_weight_from_models() {
    let s = shape::Model {
        id: 1,
        created_at: Some(Utc::now()),
        updated_at: Some(Utc::now()),
        name: "Steel Plate".into(),
        openscad_code: "cube([length, width, thickness]);".into(),
        variable_names: serde_json::json!(["length", "width", "thickness"]),
    };

    let m = material::Model {
        id: 1,
        created_at: Some(Utc::now()),
        updated_at: Some(Utc::now()),
        name: "Mild Steel".into(),
        density: 7850.0, // kg/m³
    };

    let mut vars = HashMap::new();
    vars.insert("length".into(), 1000.0);   // 1000mm = 1m
    vars.insert("width".into(), 1000.0);    // 1000mm = 1m
    vars.insert("thickness".into(), 10.0);  // 10mm thickness

    let weight = component::Model::get_weight_from_models(&s, &m, vars);
    // Vol = 1000 * 1000 * 10 mm3 = 0.01 m3, Weight = 0.01 * 7850 = 78.5 kg
    assert!((weight - 78.5).abs() < 1e-3, "Expected 78.5 kg, got {}", weight);
}

#[test]
fn test_machine_rate_rupees_paisa() {
    let m = machine::Model {
        id: 1,
        created_at: None,
        updated_at: None,
        name: "CNC VMC".into(),
        rate_decimal: Decimal::from_str_exact("1250.75").unwrap(),
    };

    let (rupees, paisa) = m.rate();
    assert_eq!(rupees, 1250);
    assert_eq!(paisa, 75);

    let round_trip = machine::rupees_paisa_to_decimal(rupees, paisa);
    assert_eq!(round_trip, Decimal::from_str_exact("1250.75").unwrap());
}

#[test]
fn test_invoice_machine_line_calculation() {
    let line = proforma_invoice_machine_line::Model {
        id: 1,
        created_at: None,
        updated_at: None,
        invoice_id: 10,
        machine_id: Some(1),
        name: "Milling Operation".into(),
        // 1.5 hours in nanoseconds = 1.5 * 3.6e12
        time_used: JobDuration::from_nanos(5_400_000_000_000),
        rate_decimal: Decimal::from(800), // Rs 800 / hr
    };

    let (r, p) = line.rate();
    assert_eq!(r, 800);
    assert_eq!(p, 0);

    let total = line.line_total();
    // 1.5 hrs * 800 = Rs 1200.00
    assert_eq!(total, Decimal::from(1200));
}

#[test]
fn test_invoice_material_line_qty_and_total() {
    let line = proforma_invoice_material_line::Model {
        id: 1,
        created_at: None,
        updated_at: None,
        invoice_id: 10,
        material_id: Some(1),
        name: "SS 304 Round Bar".into(),
        rate_decimal: Decimal::from_str_exact("420.50").unwrap(),
        qty_decimal: Decimal::from_str_exact("2.500").unwrap(), // 2 kg 500 g
    };

    let (r, p) = line.rate();
    assert_eq!(r, 420);
    assert_eq!(p, 50);

    // qty() clamps grams to 255 due to u8 limitation in spec
    let (kg, g_clamped) = line.qty();
    assert_eq!(kg, 2);
    assert_eq!(g_clamped, 255);

    // qty_full_grams() gives true 500 grams
    let (kg2, g_full) = line.qty_full_grams();
    assert_eq!(kg2, 2);
    assert_eq!(g_full, 500);

    // Line total: 2.500 * 420.50 = 1051.25
    assert_eq!(line.line_total(), Decimal::from_str_exact("1051.25").unwrap());
}

#[test]
fn test_analytical_primitives() {
    let mut vars = HashMap::new();
    vars.insert("diameter".into(), 100.0); // 100mm diameter
    vars.insert("length".into(), 1000.0);  // 1000mm length (1m)

    let vol = PrimitiveKind::Cylinder.calculate_volume(&vars).unwrap();
    let expected = std::f64::consts::PI * 0.05 * 0.05 * 1.0;
    assert!((vol - expected).abs() < 1e-6);
}

#[tokio::test]
async fn test_work_orders_migration_on_sqlite() {
    use sea_orm_migration::MigratorTrait;
    use kds_tagore_rs::work_orders::migrations::Migrator;

    let db = sea_orm::Database::connect("sqlite::memory:")
        .await
        .expect("connect sqlite memory");
    Migrator::up(&db, None).await.expect("migrator up");
}

#[tokio::test]
async fn test_work_orders_list_handler_with_capabilities() {
    use axum::http::Uri;
    use kds_tagore_rs::work_orders::{
        handlers, migrations::Migrator, state::WorkOrdersState,
    };
    use lariv_rs::{
        components::SharedChromeFolder,
        http::Cap,
        plugins::users::middleware::OptionalAuth,
        web::Htmx,
    };
    use sea_orm_migration::MigratorTrait;

    let db = sea_orm::Database::connect("sqlite::memory:")
        .await
        .expect("connect sqlite memory");
    Migrator::up(&db, None).await.expect("migrator up");

    let state = WorkOrdersState::new(db);
    struct DummyFolder;
    impl lariv_rs::components::FoldChrome for DummyFolder {
        fn fold(&self, _ctx: &lariv_rs::components::SlotCtx) -> lariv_rs::components::ShellChrome {
            lariv_rs::components::ShellChrome::default()
        }
    }
    let chrome: SharedChromeFolder = std::sync::Arc::new(DummyFolder);
    let markup = handlers::work_orders_list(
        Cap(state),
        Cap(chrome),
        OptionalAuth(None),
        Htmx::default(),
        Uri::from_static("/work-orders/"),
    ).await;

    let html_str = markup.into_string();
    assert!(html_str.contains("Work Orders"));
}

#[tokio::test]
async fn test_standard_shapes_seeded_and_visible_in_ui() {
    use axum::http::Uri;
    use kds_tagore_rs::work_orders::{
        handlers, migrations::Migrator, state::WorkOrdersState,
    };
    use lariv_rs::{
        components::SharedChromeFolder,
        http::Cap,
        plugins::users::middleware::OptionalAuth,
        web::Htmx,
    };
    use sea_orm_migration::MigratorTrait;

    let db = sea_orm::Database::connect("sqlite::memory:")
        .await
        .expect("connect sqlite memory");
    Migrator::up(&db, None).await.expect("migrator up");

    let state = WorkOrdersState::new(db);
    struct DummyFolder;
    impl lariv_rs::components::FoldChrome for DummyFolder {
        fn fold(&self, _ctx: &lariv_rs::components::SlotCtx) -> lariv_rs::components::ShellChrome {
            lariv_rs::components::ShellChrome::default()
        }
    }
    let chrome: SharedChromeFolder = std::sync::Arc::new(DummyFolder);

    // 1. Check shapes list page
    let markup = handlers::shapes_list(
        Cap(state),
        Cap(chrome.clone()),
        OptionalAuth(None),
        Htmx::default(),
        Uri::from_static("/work-orders/shapes"),
    ).await;

    let html_str = markup.into_string();
    assert!(html_str.contains("Box / Plate"), "HTML must contain Box / Plate");
    assert!(html_str.contains("Cylinder / Round Bar"), "HTML must contain Cylinder");
    assert!(html_str.contains("Standard Stock"), "HTML must contain Standard Stock badge");

    // 2. Check shape create modal contains preset toolbar
    let modal_markup = handlers::shape_create_get(
        Cap(chrome),
        OptionalAuth(None),
        axum::extract::Query(lariv_rs::web::ModalFormQuery::default()),
    ).await;
    let modal_html = modal_markup.into_string();
    assert!(modal_html.contains("Standard Presets"), "Modal must contain presets header");
    assert!(modal_html.contains("Box / Plate"), "Modal must contain preset button");
    assert!(modal_html.contains("data-list-row-input"), "Modal must contain string list input");
    assert!(!modal_html.contains("comma-separated"), "Modal must not contain csv tooltip/label");
}

#[tokio::test]
async fn test_shape_create_post_with_string_list() {
    use kds_tagore_rs::work_orders::{
        entities::shape, forms::ShapeForm, handlers, migrations::Migrator, state::WorkOrdersState,
    };
    use lariv_rs::{
        components::SharedChromeFolder, html_form::HtmlFormBody, http::Cap,
        plugins::users::middleware::OptionalAuth, web::Htmx,
    };
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    use sea_orm_migration::MigratorTrait;

    let db = sea_orm::Database::connect("sqlite::memory:")
        .await
        .expect("connect sqlite memory");
    Migrator::up(&db, None).await.expect("migrator up");

    let state = WorkOrdersState::new(db.clone());
    struct DummyFolder;
    impl lariv_rs::components::FoldChrome for DummyFolder {
        fn fold(&self, _ctx: &lariv_rs::components::SlotCtx) -> lariv_rs::components::ShellChrome {
            lariv_rs::components::ShellChrome::default()
        }
    }
    let chrome: SharedChromeFolder = std::sync::Arc::new(DummyFolder);

    let res = handlers::shape_create_post(
        Cap(state.clone()),
        Cap(chrome.clone()),
        OptionalAuth(None),
        Htmx::default(),
        axum::extract::Query(lariv_rs::web::ModalFormQuery::default()),
        HtmlFormBody(ShapeForm {
            name: "Custom Pentagon Bar".into(),
            variables: vec!["side".into(), "length".into()],
            openscad_code: "cylinder(r=side, h=length, $fn=5);".into(),
        }),
    ).await;
    assert_eq!(res.status(), axum::http::StatusCode::SEE_OTHER);

    let saved = shape::Entity::find()
        .filter(shape::Column::Name.eq("Custom Pentagon Bar"))
        .one(&db)
        .await
        .expect("query custom shape")
        .expect("custom shape exists in db");
    assert_eq!(saved.name, "Custom Pentagon Bar");
    assert_eq!(saved.variable_names_vec(), vec!["side", "length"]);
}

#[tokio::test]
async fn test_shape_analytical_volume_calculation() {
    use kds_tagore_rs::work_orders::entities::shape;
    use std::collections::HashMap;

    let cylinder = shape::Model {
        id: 1,
        created_at: None,
        updated_at: None,
        name: "Cylinder / Round Bar".into(),
        openscad_code: "cylinder(h=length, r=diameter/2);".into(),
        variable_names: serde_json::json!(["diameter", "length"]),
    };

    assert!(cylinder.is_standard(), "Cylinder must be recognized as standard shape");

    let mut vars = HashMap::new();
    vars.insert("diameter".into(), 100.0); // 100 mm
    vars.insert("length".into(), 1000.0);  // 1000 mm = 1 m

    let volume = cylinder.get_volume(vars);
    let expected = std::f64::consts::PI * 0.05 * 0.05 * 1.0;
    assert!((volume - expected).abs() < 1e-6, "Volume must match analytical formula");
}

#[test]
fn test_component_fixed_variables_and_free_variables() {
    let mut fixed = serde_json::Map::new();
    fixed.insert("width".into(), serde_json::json!(2.5));
    fixed.insert("thickness".into(), serde_json::json!(3.5));

    let c = component::Model {
        id: 1,
        created_at: None,
        updated_at: None,
        name: "2.5x3.5mm Bar".into(),
        shape_id: 1,
        material_id: 1,
        fixed_variables: serde_json::Value::Object(fixed),
    };

    let shape = shape::Model {
        id: 1,
        created_at: None,
        updated_at: None,
        name: "Box / Plate".into(),
        openscad_code: "cube([length, width, thickness]);".into(),
        variable_names: serde_json::json!(["length", "width", "thickness"]),
    };

    let fixed_map = c.fixed_variables_map();
    assert_eq!(fixed_map.get("width"), Some(&2.5));
    assert_eq!(fixed_map.get("thickness"), Some(&3.5));

    let free = c.free_variable_names(&shape);
    assert_eq!(free, vec!["length"]);
}

#[test]
fn test_solve_final_dimension_from_weight_analytical() {
    // 2.5mm x 3.5mm bar, Mild Steel (7850 kg/m3)
    // Desired length = 1000 mm (1 m)
    // Volume = 1000 * 2.5 * 3.5 = 8750 mm3 = 8.75e-6 m3
    // Weight = 8.75e-6 * 7850 = 0.0686875 kg
    let mut fixed = serde_json::Map::new();
    fixed.insert("width".into(), serde_json::json!(2.5));
    fixed.insert("thickness".into(), serde_json::json!(3.5));

    let c = component::Model {
        id: 1,
        created_at: None,
        updated_at: None,
        name: "2.5x3.5mm Bar".into(),
        shape_id: 1,
        material_id: 1,
        fixed_variables: serde_json::Value::Object(fixed),
    };

    let s = shape::Model {
        id: 1,
        created_at: None,
        updated_at: None,
        name: "Box / Plate".into(),
        openscad_code: "cube([length, width, thickness]);".into(),
        variable_names: serde_json::json!(["length", "width", "thickness"]),
    };

    let m = material::Model {
        id: 1,
        created_at: None,
        updated_at: None,
        name: "Mild Steel".into(),
        density: 7850.0,
    };

    let (free_name, solved_dim) = c.solve_final_variable_from_weight(&s, &m, 0.0686875).expect("solve dimension");
    assert_eq!(free_name, "length");
    assert!((solved_dim - 1000.0).abs() < 1e-3, "Expected length 1000 mm, got {}", solved_dim);
}

#[test]
fn test_solve_final_dimension_from_cost() {
    let mut fixed = serde_json::Map::new();
    fixed.insert("width".into(), serde_json::json!(2.5));
    fixed.insert("thickness".into(), serde_json::json!(3.5));

    let c = component::Model {
        id: 1,
        created_at: None,
        updated_at: None,
        name: "2.5x3.5mm Bar".into(),
        shape_id: 1,
        material_id: 1,
        fixed_variables: serde_json::Value::Object(fixed),
    };

    let s = shape::Model {
        id: 1,
        created_at: None,
        updated_at: None,
        name: "Box / Plate".into(),
        openscad_code: "cube([length, width, thickness]);".into(),
        variable_names: serde_json::json!(["length", "width", "thickness"]),
    };

    let m = material::Model {
        id: 1,
        created_at: None,
        updated_at: None,
        name: "Mild Steel".into(),
        density: 7850.0,
    };

    let r = kds_tagore_rs::work_orders::entities::material_rate::Model {
        id: 1,
        created_at: None,
        updated_at: None,
        material_id: 1,
        rate_decimal: rust_decimal::Decimal::new(8500, 2), // ₹85.00 / kg
        datetime: Utc::now(),
    };

    // Weight 0.0686875 kg * 85 INR/kg = 5.8384375 INR
    let (free_name, solved_dim) = c.solve_final_variable_from_cost(&s, &m, r.rate(), 5.8384375).expect("solve dimension from cost");
    assert_eq!(free_name, "length");
    assert!((solved_dim - 1000.0).abs() < 1e-3, "Expected length 1000 mm, got {}", solved_dim);
}

#[tokio::test]
async fn test_work_order_and_lines_creation_and_total_calculation() {
    use std::str::FromStr;
    use kds_tagore_rs::work_orders::{
        entities::{work_order, work_order_line},
        handlers::{self, WorkOrderCreateForm},
        migrations::Migrator,
        state::WorkOrdersState,
    };
    use lariv_rs::{components::SharedChromeFolder, http::Cap};
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
    use sea_orm_migration::MigratorTrait;

    let db = sea_orm::Database::connect("sqlite::memory:").await.expect("sqlite db");
    Migrator::up(&db, None).await.expect("migrator up");

    let state = WorkOrdersState::new(db.clone());
    struct DummyFolder;
    impl lariv_rs::components::FoldChrome for DummyFolder {
        fn fold(&self, _ctx: &lariv_rs::components::SlotCtx) -> lariv_rs::components::ShellChrome {
            lariv_rs::components::ShellChrome::default()
        }
    }
    let chrome: SharedChromeFolder = std::sync::Arc::new(DummyFolder);

    // Ensure seeds exist for shapes, materials, rates, and components
    kds_tagore_rs::work_orders::seed::ensure_standard_seeds(&db).await.expect("seeds");
    let comps = kds_tagore_rs::work_orders::entities::component::Entity::find()
        .order_by_asc(kds_tagore_rs::work_orders::entities::component::Column::Id)
        .all(&db)
        .await
        .expect("query comps");
    let comp1 = &comps[0];
    let comp2 = &comps[1];

    // 0. Verify WorkOrder create modal renders inline items widget
    let create_modal_res = handlers::work_order_create_get(
        Cap(state.clone()),
        Cap(chrome.clone()),
        lariv_rs::plugins::users::middleware::OptionalAuth(None),
        axum::extract::Query(lariv_rs::web::ModalFormQuery::default()),
    ).await;
    let create_modal_html = create_modal_res.into_string();
    assert!(create_modal_html.contains("Draft Work Order"), "Work Order form must render Draft Work Order");
    assert!(create_modal_html.contains("Add Line"), "Work Order form must have Add Line button");
    assert!(create_modal_html.contains("Dimensions / Variables"), "Work Order items table must have Dimensions column");
    assert!(create_modal_html.contains("Final Cost"), "Work Order items table must have Final Cost column");

    // 1. Create a Work Order with Order Number, Customer ID, and inline items
    // comp1 is MS Flat Bar 2.5x3.5mm with 1 free variable "length"
    // comp2 is SS 304 Round Rod Ø20mm with 1 free variable "length"
    let inline_items_json = format!(r#"[
        {{"component_id": {}, "variables": {{"length": 1000}}, "quantity": "10", "extra_data": {{"finish": "zinc"}}}},
        {{"component_id": {}, "mode": "weight", "target_weight": 2.49, "quantity": "4", "extra_data": {{"lot": "A-1"}}}}
    ]"#, comp1.id, comp2.id);

    let form = WorkOrderCreateForm {
        order_number: "WO-TEST-001".into(),
        customer_id: 42,
        items: Some(inline_items_json),
        machine_lines: None,
    };

    let res = handlers::work_order_create_post(
        Cap(state.clone()),
        Cap(chrome.clone()),
        lariv_rs::plugins::users::middleware::OptionalAuth(None),
        lariv_rs::web::Htmx::default(),
        axum::extract::Query(lariv_rs::web::ModalFormQuery::default()),
        axum::extract::Form(form),
    ).await;

    let (parts, _body) = axum::response::IntoResponse::into_response(res).into_parts();
    assert_eq!(parts.status, axum::http::StatusCode::SEE_OTHER);

    let created_order = work_order::Entity::find()
        .filter(work_order::Column::OrderNumber.eq("WO-TEST-001"))
        .one(&db)
        .await
        .expect("query order")
        .expect("order exists");
    assert_eq!(created_order.order_number, "WO-TEST-001");
    assert_eq!(created_order.customer_id, 42);

    // 2. Verify inline items were persisted with foreign key draft_work_order_id = created_order.id
    let lines = work_order_line::Entity::find()
        .filter(work_order_line::Column::DraftWorkOrderId.eq(created_order.id))
        .order_by_asc(work_order_line::Column::Id)
        .all(&db)
        .await
        .expect("query lines");
    assert_eq!(lines.len(), 2, "Both inline items must be saved to DB");

    let l1 = &lines[0];
    assert_eq!(l1.draft_work_order_id, created_order.id);
    assert_eq!(l1.component_id, comp1.id);
    assert_eq!(l1.quantity, Decimal::from_str("10").unwrap());
    assert!(l1.unit_weight > Decimal::ZERO);
    assert!(l1.material_rate > Decimal::ZERO);
    // Cost formula: quantity * material.rate * component.weight
    let expected_c1 = (l1.quantity * l1.material_rate * l1.unit_weight).round_dp(2);
    assert_eq!(l1.final_cost, expected_c1);
    assert_eq!(l1.line_total(), l1.final_cost);
    assert!(l1.extra_data_str().contains("zinc"));

    let l2 = &lines[1];
    assert_eq!(l2.draft_work_order_id, created_order.id);
    assert_eq!(l2.component_id, comp2.id);
    assert_eq!(l2.quantity, Decimal::from_str("4").unwrap());
    assert!(l2.unit_weight > Decimal::ZERO);
    assert!(l2.material_rate > Decimal::ZERO);
    let expected_c2 = (l2.quantity * l2.material_rate * l2.unit_weight).round_dp(2);
    assert_eq!(l2.final_cost, expected_c2);
    assert_eq!(l2.line_total(), l2.final_cost);
    assert!(l2.extra_data_str().contains("A-1"));

    // Total amount calculation (sum of line final costs)
    let total_amount = created_order.total_amount(&lines);
    assert_eq!(total_amount, l1.final_cost + l2.final_cost);

    // 3. Test component FK picker route
    let comp_picker_res = handlers::component_select(
        Cap(state.clone()),
        lariv_rs::web::Htmx::default(),
        "/work-orders/components/pick".parse().unwrap(),
        axum::extract::Query(handlers::EntitySelectQuery {
            target_input: Some("component_id".into()),
            name: Some("Flat Bar".into()),
        }),
    ).await;
    let comp_picker_html = comp_picker_res.into_string();
    assert!(comp_picker_html.contains("MS Flat Bar"), "Component picker must return matching component");

    // Verify the 2 inline lines exist
    let lines_after = work_order_line::Entity::find()
        .filter(work_order_line::Column::DraftWorkOrderId.eq(created_order.id))
        .all(&db)
        .await
        .expect("query lines");
    assert_eq!(lines_after.len(), 2);

    // 4. Verify Work Order Detail Page renders lines and grand total
    let detail_res = handlers::work_order_detail(
        Cap(state.clone()),
        Cap(chrome.clone()),
        lariv_rs::plugins::users::middleware::OptionalAuth(None),
        lariv_rs::web::Htmx::default(),
        axum::extract::Path(created_order.id),
    ).await;
    let body = axum::body::to_bytes(detail_res.into_body(), usize::MAX).await.expect("read body");
    let html = String::from_utf8_lossy(&body);
    assert!(html.contains("WO-TEST-001"), "Detail page must show order number");
    assert!(html.contains("MS Flat Bar"), "Detail page must list component name");
    assert!(html.contains("SS 304 Round Rod"), "Detail page must list component name 2");
}


#[tokio::test]
async fn test_all_entities_edit_get_and_post() {
    use std::str::FromStr;
    use axum::response::IntoResponse;
    use kds_tagore_rs::work_orders::{
        entities::{component, machine, material, proforma_invoice, shape, work_order, work_order_line},
        handlers::{
            self, ComponentEditForm, InvoiceEditForm, MachineEditForm, MaterialEditForm,
            ShapeEditForm, WorkOrderEditForm,
        },
        migrations::Migrator,
        seed::ensure_standard_seeds,
        state::WorkOrdersState,
    };
    use lariv_rs::{
        components::SharedChromeFolder, html_form::HtmlFormBody, http::Cap,
        plugins::users::middleware::OptionalAuth,
    };
    use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
    use sea_orm_migration::MigratorTrait;

    let db = sea_orm::Database::connect("sqlite::memory:").await.expect("sqlite db");
    Migrator::up(&db, None).await.expect("migrator up");
    ensure_standard_seeds(&db).await.expect("seed");

    let state = WorkOrdersState::new(db.clone());
    struct DummyFolder;
    impl lariv_rs::components::FoldChrome for DummyFolder {
        fn fold(&self, _ctx: &lariv_rs::components::SlotCtx) -> lariv_rs::components::ShellChrome {
            lariv_rs::components::ShellChrome::default()
        }
    }
    let chrome: SharedChromeFolder = std::sync::Arc::new(DummyFolder);

    // 1. Shape Edit GET & POST
    let box_shape = shape::Entity::find()
        .filter(shape::Column::Name.eq("Box / Plate"))
        .one(&db)
        .await
        .expect("query shape")
        .expect("seeded Box / Plate exists");

    let res = handlers::shape_edit_get(
        Cap(state.clone()),
        Cap(chrome.clone()),
        OptionalAuth(None),
        axum::extract::Query(lariv_rs::web::ModalFormQuery::default()),
        axum::extract::Path(box_shape.id),
    ).await;
    assert_eq!(res.status(), axum::http::StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.expect("read body");
    let html = String::from_utf8_lossy(&body);
    assert!(html.contains("Edit Shape"), "Modal HTML must contain 'Edit Shape'");
    assert!(html.contains("Box / Plate"), "Modal HTML must contain existing name");
    assert!(html.contains("data-list-row-input"), "Modal HTML must contain string list input");
    assert!(!html.contains("comma-separated"), "Modal HTML must not contain csv tooltip/label");

    let res = handlers::shape_edit_post(
        Cap(state.clone()),
        Cap(chrome.clone()),
        OptionalAuth(None),
        lariv_rs::web::Htmx::default(),
        axum::extract::Query(lariv_rs::web::ModalFormQuery::default()),
        axum::extract::Path(box_shape.id),
        HtmlFormBody(ShapeEditForm {
            name: "Box / Plate Modified".into(),
            openscad_code: "cube([length, width, thickness]);".into(),
            variables: vec!["length".into(), "width".into(), "thickness".into()],
        }),
    ).await.into_response();
    assert_eq!(res.status(), axum::http::StatusCode::SEE_OTHER);

    let updated_shape = shape::Entity::find_by_id(box_shape.id)
        .one(&db)
        .await
        .expect("query shape")
        .expect("shape exists");
    assert_eq!(updated_shape.name, "Box / Plate Modified");
    assert_eq!(updated_shape.variable_names_vec(), vec!["length", "width", "thickness"]);

    // 2. Material Edit GET & POST
    let ms_mat = material::Entity::find()
        .filter(material::Column::Name.contains("Mild Steel"))
        .one(&db)
        .await
        .expect("query material")
        .expect("seeded Mild Steel exists");

    let res = handlers::material_edit_get(
        Cap(state.clone()),
        Cap(chrome.clone()),
        OptionalAuth(None),
        axum::extract::Query(lariv_rs::web::ModalFormQuery::default()),
        axum::extract::Path(ms_mat.id),
    ).await;
    assert_eq!(res.status(), axum::http::StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.expect("read body");
    let html = String::from_utf8_lossy(&body);
    assert!(html.contains("Edit Material"), "Modal HTML must contain 'Edit Material'");
    assert!(html.contains("Mild Steel"), "Modal HTML must contain existing material name");

    let res = handlers::material_edit_post(
        Cap(state.clone()),
        Cap(chrome.clone()),
        OptionalAuth(None),
        lariv_rs::web::Htmx::default(),
        axum::extract::Query(lariv_rs::web::ModalFormQuery::default()),
        axum::extract::Path(ms_mat.id),
        axum::extract::Form(MaterialEditForm {
            name: "Mild Steel IS2062".into(),
            density: 7850.0,
        }),
    ).await.into_response();
    assert_eq!(res.status(), axum::http::StatusCode::SEE_OTHER);

    let updated_mat = material::Entity::find_by_id(ms_mat.id)
        .one(&db)
        .await
        .expect("query mat")
        .expect("mat exists");
    assert_eq!(updated_mat.name, "Mild Steel IS2062");

    // 3. Machine Edit GET & POST
    let new_mach = machine::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(Utc::now())),
        updated_at: Set(Some(Utc::now())),
        name: Set("CNC Milling 3-Axis".into()),
        rate_decimal: Set(rust_decimal::Decimal::new(120000, 2)), // ₹1200.00 / hr
    };
    let mach = new_mach.insert(&db).await.expect("insert machine");

    let res = handlers::machine_edit_get(
        Cap(state.clone()),
        Cap(chrome.clone()),
        OptionalAuth(None),
        axum::extract::Query(lariv_rs::web::ModalFormQuery::default()),
        axum::extract::Path(mach.id),
    ).await;
    assert_eq!(res.status(), axum::http::StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.expect("read body");
    let html = String::from_utf8_lossy(&body);
    assert!(html.contains("Edit Machine"), "Modal HTML must contain 'Edit Machine'");
    assert!(html.contains("CNC Milling 3-Axis"), "Modal HTML must contain machine name");

    let res = handlers::machine_edit_post(
        Cap(state.clone()),
        Cap(chrome.clone()),
        OptionalAuth(None),
        lariv_rs::web::Htmx::default(),
        axum::extract::Query(lariv_rs::web::ModalFormQuery::default()),
        axum::extract::Path(mach.id),
        axum::extract::Form(MachineEditForm {
            name: "CNC Milling 5-Axis".into(),
            rate: 1500.0,
        }),
    ).await.into_response();
    assert_eq!(res.status(), axum::http::StatusCode::SEE_OTHER);

    let updated_mach = machine::Entity::find_by_id(mach.id)
        .one(&db)
        .await
        .expect("query mach")
        .expect("mach exists");
    assert_eq!(updated_mach.name, "CNC Milling 5-Axis");
    assert_eq!(updated_mach.rate_decimal, rust_decimal::Decimal::new(1500, 0));

    // 4. Component Edit GET & POST
    let bar_comp = component::Entity::find()
        .filter(component::Column::Name.contains("2.5x3.5mm"))
        .one(&db)
        .await
        .expect("query comp")
        .expect("seeded component exists");

    let res = handlers::component_edit_get(
        Cap(state.clone()),
        Cap(chrome.clone()),
        OptionalAuth(None),
        axum::extract::Query(lariv_rs::web::ModalFormQuery::default()),
        axum::extract::Path(bar_comp.id),
    ).await;
    assert_eq!(res.status(), axum::http::StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.expect("read body");
    let html = String::from_utf8_lossy(&body);
    assert!(html.contains("Edit Component"), "Modal HTML must contain 'Edit Component'");
    assert!(html.contains("2.5x3.5mm"), "Modal HTML must contain component name");
    assert!(html.contains("/work-orders/shapes/pick"), "Shape must be an fkey picker");
    assert!(html.contains("/work-orders/materials/pick"), "Material must be an fkey picker");
    assert!(html.contains("data-kv-key-input"), "Fixed dimensions must use KV list key input");
    assert!(!html.contains("<select name=\"shape_id\""), "Shape must not be a dropdown");
    assert!(!html.contains("<select name=\"material_id\""), "Material must not be a dropdown");
    assert!(!html.contains("<textarea name=\"fixed_variables\""), "Fixed dimensions must not be a raw textarea");
    let pos = html.find("allowedKeys").expect("allowedKeys must exist in edit page");
    let allowed_keys_snippet = &html[pos..pos + 80];
    assert!(
        allowed_keys_snippet.contains("length")
            && allowed_keys_snippet.contains("width")
            && allowed_keys_snippet.contains("thickness"),
        "Edit page must filter allowedKeys strictly for current shape (Box), got: {}",
        allowed_keys_snippet
    );
    assert!(
        !allowed_keys_snippet.contains("diameter") && !allowed_keys_snippet.contains("across_flats"),
        "Box allowedKeys must not contain diameter or across_flats, got: {}",
        allowed_keys_snippet
    );

    let res = handlers::component_edit_post(
        Cap(state.clone()),
        Cap(chrome.clone()),
        OptionalAuth(None),
        lariv_rs::web::Htmx::default(),
        axum::extract::Query(lariv_rs::web::ModalFormQuery::default()),
        axum::extract::Path(bar_comp.id),
        HtmlFormBody(ComponentEditForm {
            name: "MS Flat Bar 3.0x4.0mm".into(),
            shape_id: bar_comp.shape_id,
            material_id: bar_comp.material_id,
            fixed_variables: Some(r#"{"width": 3.0, "thickness": 4.0}"#.into()),
        }),
    ).await.into_response();
    assert_eq!(res.status(), axum::http::StatusCode::SEE_OTHER);

    let updated_comp = component::Entity::find_by_id(bar_comp.id)
        .one(&db)
        .await
        .expect("query comp")
        .expect("comp exists");
    assert_eq!(updated_comp.name, "MS Flat Bar 3.0x4.0mm");
    let fixed_map = updated_comp.fixed_variables_map();
    assert_eq!(fixed_map.get("width"), Some(&3.0));
    assert_eq!(fixed_map.get("thickness"), Some(&4.0));

    // 5. Work Order Edit GET & POST
    let new_order = work_order::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(Utc::now())),
        updated_at: Set(Some(Utc::now())),
        order_number: Set("WO-EDIT-001".into()),
        customer_id: Set(101),
    };
    let order = new_order.insert(&db).await.expect("insert order");

    let res = handlers::work_order_edit_get(
        Cap(state.clone()),
        Cap(chrome.clone()),
        OptionalAuth(None),
        axum::extract::Query(lariv_rs::web::ModalFormQuery::default()),
        axum::extract::Path(order.id),
    ).await;
    assert_eq!(res.status(), axum::http::StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.expect("read body");
    let html = String::from_utf8_lossy(&body);
    assert!(html.contains("Draft Work Order"), "Modal HTML must contain 'Draft Work Order'");
    assert!(html.contains("WO-EDIT-001"), "Modal HTML must contain order number");
    assert!(html.contains("Draft Work Order Material Lines"), "Modal HTML must contain Draft Work Order Material Lines");

    let res = handlers::work_order_edit_post(
        Cap(state.clone()),
        Cap(chrome.clone()),
        OptionalAuth(None),
        lariv_rs::web::Htmx::default(),
        axum::extract::Query(lariv_rs::web::ModalFormQuery::default()),
        axum::extract::Path(order.id),
        axum::extract::Form(WorkOrderEditForm {
            order_number: "WO-EDIT-001-REV1".into(),
            customer_id: 101,
            items: Some(format!(r#"[
                {{"component_id": {}, "variables": {{"length": 500}}, "quantity": "5", "extra_data": null}}
            ]"#, bar_comp.id)),
            machine_lines: None,
        }),
    ).await.into_response();
    assert_eq!(res.status(), axum::http::StatusCode::SEE_OTHER);

    let updated_order = work_order::Entity::find_by_id(order.id)
        .one(&db)
        .await
        .expect("query order")
        .expect("order exists");
    assert_eq!(updated_order.order_number, "WO-EDIT-001-REV1");

    let edited_lines = work_order_line::Entity::find()
        .filter(work_order_line::Column::DraftWorkOrderId.eq(order.id))
        .all(&db)
        .await
        .expect("query lines");
    assert_eq!(edited_lines.len(), 1);
    assert_eq!(edited_lines[0].component_id, bar_comp.id);
    assert_eq!(edited_lines[0].quantity, Decimal::from_str("5").unwrap());
    assert!(edited_lines[0].final_cost > Decimal::ZERO);

    // 6. Proforma Invoice Edit GET & POST
    let new_invoice = proforma_invoice::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(Utc::now())),
        updated_at: Set(Some(Utc::now())),
        invoice_number: Set("PI-EDIT-100".into()),
        date: Set(chrono::NaiveDate::from_ymd_opt(2026, 9, 14).unwrap()),
        customer_id: Set(101),
        work_order_id: Set(Some(order.id)),
    };
    let invoice = new_invoice.insert(&db).await.expect("insert invoice");

    let res = handlers::invoice_edit_get(
        Cap(state.clone()),
        Cap(chrome.clone()),
        OptionalAuth(None),
        axum::extract::Query(lariv_rs::web::ModalFormQuery::default()),
        axum::extract::Path(invoice.id),
    ).await;
    assert_eq!(res.status(), axum::http::StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.expect("read body");
    let html = String::from_utf8_lossy(&body);
    assert!(html.contains("Edit Proforma Invoice"), "Modal HTML must contain 'Edit Proforma Invoice'");
    assert!(html.contains("PI-EDIT-100"), "Modal HTML must contain invoice number");

    let res = handlers::invoice_edit_post(
        Cap(state.clone()),
        Cap(chrome.clone()),
        OptionalAuth(None),
        lariv_rs::web::Htmx::default(),
        axum::extract::Query(lariv_rs::web::ModalFormQuery::default()),
        axum::extract::Path(invoice.id),
        axum::extract::Form(InvoiceEditForm {
            invoice_number: "PI-EDIT-100-FINAL".into(),
            date: "2026-09-15".into(),
            customer_id: 101,
            work_order_id: Some(order.id),
        }),
    ).await.into_response();
    assert_eq!(res.status(), axum::http::StatusCode::SEE_OTHER);

    let updated_invoice = proforma_invoice::Entity::find_by_id(invoice.id)
        .one(&db)
        .await
        .expect("query invoice")
        .expect("invoice exists");
    assert_eq!(updated_invoice.invoice_number, "PI-EDIT-100-FINAL");
    assert_eq!(updated_invoice.date, chrono::NaiveDate::from_ymd_opt(2026, 9, 15).unwrap());
}

#[tokio::test]
async fn test_detail_pages_have_edit_button() {
    use kds_tagore_rs::work_orders::{
        entities::{machine, proforma_invoice, work_order},
        handlers,
        migrations::Migrator,
        seed::ensure_standard_seeds,
        state::WorkOrdersState,
    };
    use lariv_rs::{components::SharedChromeFolder, http::Cap, plugins::users::middleware::OptionalAuth, web::Htmx};
    use sea_orm::{ActiveModelTrait, Set};
    use sea_orm_migration::MigratorTrait;

    let db = sea_orm::Database::connect("sqlite::memory:").await.expect("sqlite db");
    Migrator::up(&db, None).await.expect("migrator up");
    ensure_standard_seeds(&db).await.expect("seed");

    let state = WorkOrdersState::new(db.clone());
    struct DummyFolder;
    impl lariv_rs::components::FoldChrome for DummyFolder {
        fn fold(&self, _ctx: &lariv_rs::components::SlotCtx) -> lariv_rs::components::ShellChrome {
            lariv_rs::components::ShellChrome::default()
        }
    }
    let chrome: SharedChromeFolder = std::sync::Arc::new(DummyFolder);

    // Shape Detail
    let res = handlers::shape_detail(
        Cap(state.clone()),
        Cap(chrome.clone()),
        OptionalAuth(None),
        Htmx::default(),
        axum::extract::Path(1),
    ).await;
    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.expect("read body");
    let html = String::from_utf8_lossy(&body);
    assert!(html.contains("Edit"), "Shape detail must have Edit button");
    assert!(html.contains("/work-orders/shapes/1/edit"), "Shape detail must link to edit URL");

    // Material Detail
    let res = handlers::material_detail(
        Cap(state.clone()),
        Cap(chrome.clone()),
        OptionalAuth(None),
        Htmx::default(),
        axum::extract::Path(1),
    ).await;
    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.expect("read body");
    let html = String::from_utf8_lossy(&body);
    assert!(html.contains("Edit"), "Material detail must have Edit button");
    assert!(html.contains("/work-orders/materials/1/edit"), "Material detail must link to edit URL");

    // Machine Detail
    let new_mach = machine::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(Utc::now())),
        updated_at: Set(Some(Utc::now())),
        name: Set("Lathe".into()),
        rate_decimal: Set(rust_decimal::Decimal::new(500, 0)),
    };
    let mach = new_mach.insert(&db).await.expect("insert machine");
    let res = handlers::machine_detail(
        Cap(state.clone()),
        Cap(chrome.clone()),
        OptionalAuth(None),
        Htmx::default(),
        axum::extract::Path(mach.id),
    ).await;
    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.expect("read body");
    let html = String::from_utf8_lossy(&body);
    assert!(html.contains("Edit"), "Machine detail must have Edit button");
    assert!(html.contains(&format!("/work-orders/machines/{}/edit", mach.id)), "Machine detail must link to edit URL");

    // Component Detail
    let res = handlers::component_detail(
        Cap(state.clone()),
        Cap(chrome.clone()),
        OptionalAuth(None),
        Htmx::default(),
        axum::extract::Path(1),
    ).await;
    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.expect("read body");
    let html = String::from_utf8_lossy(&body);
    assert!(html.contains("Edit"), "Component detail must have Edit button");
    assert!(html.contains("/work-orders/components/1/edit"), "Component detail must link to edit URL");

    // Work Order Detail
    let new_order = work_order::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(Utc::now())),
        updated_at: Set(Some(Utc::now())),
        order_number: Set("WO-DET-01".into()),
        customer_id: Set(1),
    };
    let order = new_order.insert(&db).await.expect("insert order");
    let res = handlers::work_order_detail(
        Cap(state.clone()),
        Cap(chrome.clone()),
        OptionalAuth(None),
        Htmx::default(),
        axum::extract::Path(order.id),
    ).await;
    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.expect("read body");
    let html = String::from_utf8_lossy(&body);
    assert!(html.contains("Edit"), "Work order detail must have Edit button");
    assert!(html.contains(&format!("/work-orders/orders/{}/edit", order.id)), "Work order detail must link to edit URL");

    // Proforma Invoice Detail
    let new_inv = proforma_invoice::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(Utc::now())),
        updated_at: Set(Some(Utc::now())),
        invoice_number: Set("PI-DET-01".into()),
        date: Set(chrono::NaiveDate::from_ymd_opt(2026, 9, 14).unwrap()),
        customer_id: Set(1),
        work_order_id: Set(Some(order.id)),
    };
    let inv = new_inv.insert(&db).await.expect("insert invoice");
    let res = handlers::invoice_detail(
        Cap(state.clone()),
        Cap(chrome.clone()),
        OptionalAuth(None),
        Htmx::default(),
        axum::extract::Path(inv.id),
    ).await;
    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.expect("read body");
    let html = String::from_utf8_lossy(&body);
    assert!(html.contains("Edit"), "Invoice detail must have Edit button");
    assert!(html.contains(&format!("/work-orders/invoices/{}/edit", inv.id)), "Invoice detail must link to edit URL");
}

#[tokio::test]
async fn test_component_form_fkey_and_kv_list_and_pickers() {
    use axum::response::IntoResponse;
    use kds_tagore_rs::work_orders::{
        entities::{component, material, shape},
        handlers::{self, ComponentCreateForm, EntitySelectQuery},
        migrations::Migrator,
        seed::ensure_standard_seeds,
        state::WorkOrdersState,
    };
    use lariv_rs::{
        components::SharedChromeFolder, html_form::HtmlFormBody, http::Cap,
        plugins::users::middleware::OptionalAuth, web::Htmx,
    };
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    use sea_orm_migration::MigratorTrait;

    let db = sea_orm::Database::connect("sqlite::memory:").await.expect("sqlite db");
    Migrator::up(&db, None).await.expect("migrations up");
    ensure_standard_seeds(&db).await.expect("seeds ok");

    let state = WorkOrdersState { db: db.clone() };
    struct DummyFolder;
    impl lariv_rs::components::FoldChrome for DummyFolder {
        fn fold(&self, _ctx: &lariv_rs::components::SlotCtx) -> lariv_rs::components::ShellChrome {
            lariv_rs::components::ShellChrome::default()
        }
    }
    let chrome: SharedChromeFolder = std::sync::Arc::new(DummyFolder);

    // 1. Verify component_create_get renders fkey pickers and KV list
    let res = handlers::component_create_get(
        Cap(state.clone()),
        Cap(chrome.clone()),
        OptionalAuth(None),
        axum::extract::Query(lariv_rs::web::ModalFormQuery::default()),
    ).await;
    let html = res.into_string();
    assert!(html.contains("New Component"), "Must contain 'New Component'");
    assert!(html.contains("/work-orders/shapes/pick"), "Shape must have FK picker route");
    assert!(html.contains("/work-orders/materials/pick"), "Material must have FK picker route");
    assert!(html.contains("data-kv-key-input"), "Must contain KV list key input with searchable dropdown");
    assert!(html.contains("data-kv-val-input"), "Must contain KV list value input");
    assert!(html.contains("data-kv-add-btn"), "Must contain Add Fixed Dimension button");
    assert!(!html.contains("<select name=\"shape_id\""), "Shape must not be a dropdown");
    assert!(!html.contains("<select name=\"material_id\""), "Material must not be a dropdown");
    assert!(!html.contains("<textarea name=\"fixed_variables\""), "Fixed dimensions must not be a raw textarea");
    assert!(!html.contains("Custom:"), "Must not allow custom variable names in UI");
    assert!(html.contains("allowedKeys"), "Must contain allowedKeys in Alpine data");
    assert!(html.contains("usedKeys"), "Must filter by usedKeys in Alpine data");
    assert!(html.contains("availableKeys"), "Must compute availableKeys excluding already specified options");

    // 2. Verify shape_select picker route
    let res = handlers::shape_select(
        Cap(state.clone()),
        Htmx::default(),
        "/work-orders/shapes/pick".parse().unwrap(),
        axum::extract::Query(EntitySelectQuery {
            target_input: Some("shape_id".into()),
            name: Some("Box".into()),
        }),
    ).await;
    let shape_picker_html = res.into_string();
    assert!(shape_picker_html.contains("Box / Plate"), "Shape picker must return matching shape");
    assert!(shape_picker_html.contains("data-table-container"), "Shape picker must render table container");

    // 3. Verify material_select picker route
    let res = handlers::material_select(
        Cap(state.clone()),
        Htmx::default(),
        "/work-orders/materials/pick".parse().unwrap(),
        axum::extract::Query(EntitySelectQuery {
            target_input: Some("material_id".into()),
            name: Some("Steel".into()),
        }),
    ).await;
    let mat_picker_html = res.into_string();
    assert!(mat_picker_html.contains("Mild Steel"), "Material picker must return matching material");
    assert!(mat_picker_html.contains("data-table-container"), "Material picker must render table container");

    // 4. Verify component_create_post with valid standard KV dimensions
    let box_shape = shape::Entity::find().filter(shape::Column::Name.contains("Box")).one(&db).await.unwrap().unwrap();
    let ms_mat = material::Entity::find().filter(material::Column::Name.contains("Mild Steel")).one(&db).await.unwrap().unwrap();

    let res = handlers::component_create_post(
        Cap(state.clone()),
        Cap(chrome.clone()),
        OptionalAuth(None),
        Htmx::default(),
        axum::extract::Query(lariv_rs::web::ModalFormQuery::default()),
        HtmlFormBody(ComponentCreateForm {
            name: "MS Plate 10x20mm".into(),
            shape_id: box_shape.id,
            material_id: ms_mat.id,
            fixed_variables: Some(r#"{"width": 10.0, "thickness": 20.0}"#.into()),
        }),
    ).await.into_response();
    assert_eq!(res.status(), axum::http::StatusCode::SEE_OTHER);

    let created_comp = component::Entity::find()
        .filter(component::Column::Name.eq("MS Plate 10x20mm"))
        .one(&db)
        .await
        .expect("query comp")
        .expect("component created");
    assert_eq!(created_comp.shape_id, box_shape.id);
    assert_eq!(created_comp.material_id, ms_mat.id);
    let map = created_comp.fixed_variables_map();
    assert_eq!(map.get("width"), Some(&10.0));
    assert_eq!(map.get("thickness"), Some(&20.0));

    // 5. Verify custom variable names are strictly disallowed and rejected on submit
    let res = handlers::component_create_post(
        Cap(state.clone()),
        Cap(chrome.clone()),
        OptionalAuth(None),
        Htmx::default(),
        axum::extract::Query(lariv_rs::web::ModalFormQuery::default()),
        HtmlFormBody(ComponentCreateForm {
            name: "Invalid Comp With Custom Var".into(),
            shape_id: box_shape.id,
            material_id: ms_mat.id,
            fixed_variables: Some(r#"{"custom_var": 15.0}"#.into()),
        }),
    ).await.into_response();
    assert_eq!(res.status(), axum::http::StatusCode::OK, "Form should re-render modal with error on custom variable");
    let err_bytes = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let err_html = String::from_utf8_lossy(&err_bytes);
    assert!(
        err_html.contains("not valid for shape") && err_html.contains("custom_var"),
        "Must display variable rejection error message for shape, got: {}",
        err_html
    );

    let not_created = component::Entity::find()
        .filter(component::Column::Name.eq("Invalid Comp With Custom Var"))
        .one(&db)
        .await
        .unwrap();
    assert!(not_created.is_none(), "Component with custom variable must not be created");
}

#[tokio::test]
async fn test_form_validation_and_pascal_case_deserialization() {
    use axum::extract::FromRequest;
    use axum::extract::Form;
    use axum::response::IntoResponse;
    use kds_tagore_rs::work_orders::{
        handlers::{self, WorkOrderCreateForm},
        migrations::Migrator,
        seed::ensure_standard_seeds,
        state::WorkOrdersState,
    };
    use lariv_rs::{
        components::SharedChromeFolder, http::Cap,
        plugins::users::middleware::OptionalAuth, web::Htmx,
    };
    use sea_orm_migration::MigratorTrait;

    // 1. Verify PascalCase deserialization matches what HTML form submits
    let wo_req = axum::http::Request::builder()
        .method("POST")
        .header(axum::http::header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(axum::body::Body::from("OrderNumber=WO-Pascal-1&CustomerID=10&Items=%5B%5D"))
        .unwrap();
    let Form(wo_form): Form<WorkOrderCreateForm> = Form::from_request(wo_req, &()).await.expect("deserialize PascalCase WorkOrderCreateForm");
    assert_eq!(wo_form.order_number, "WO-Pascal-1");
    assert_eq!(wo_form.customer_id, 10);

    // Empty fields deserialized safely without 422
    let req_empty = axum::http::Request::builder()
        .method("POST")
        .header(axum::http::header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(axum::body::Body::from("OrderNumber=&CustomerID="))
        .unwrap();
    let Form(empty_wo): Form<WorkOrderCreateForm> = Form::from_request(req_empty, &()).await.expect("deserialize empty form fields");
    assert_eq!(empty_wo.order_number, "");
    assert_eq!(empty_wo.customer_id, 0);

    // 2. Test server validation and error banner rendering
    let db = sea_orm::Database::connect("sqlite::memory:").await.expect("sqlite db");
    Migrator::up(&db, None).await.expect("migrations up");
    ensure_standard_seeds(&db).await.expect("seeds ok");

    let state = WorkOrdersState { db: db.clone() };
    struct DummyFolder;
    impl lariv_rs::components::FoldChrome for DummyFolder {
        fn fold(&self, _ctx: &lariv_rs::components::SlotCtx) -> lariv_rs::components::ShellChrome {
            lariv_rs::components::ShellChrome::default()
        }
    }
    let chrome: SharedChromeFolder = std::sync::Arc::new(DummyFolder);

    // Empty Order Number should re-render modal with alert-error
    let res = handlers::work_order_create_post(
        Cap(state.clone()),
        Cap(chrome.clone()),
        OptionalAuth(None),
        Htmx::default(),
        axum::extract::Query(lariv_rs::web::ModalFormQuery::default()),
        Form(WorkOrderCreateForm {
            order_number: "".into(),
            customer_id: 1,
            items: None,
            machine_lines: None,
        }),
    ).await.into_response();
    assert_eq!(res.status(), axum::http::StatusCode::OK);
    let html = String::from_utf8_lossy(&axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap()).into_owned();
    assert!(html.contains("alert-error"), "Must contain DaisyUI alert-error banner");
    assert!(html.contains("Order number is required."), "Must display order number required message");

    // Missing/0 Customer ID should re-render modal with alert-error
    let res = handlers::work_order_create_post(
        Cap(state.clone()),
        Cap(chrome.clone()),
        OptionalAuth(None),
        Htmx::default(),
        axum::extract::Query(lariv_rs::web::ModalFormQuery::default()),
        Form(WorkOrderCreateForm {
            order_number: "WO-999".into(),
            customer_id: 0,
            items: None,
            machine_lines: None,
        }),
    ).await.into_response();
    assert_eq!(res.status(), axum::http::StatusCode::OK);
    let html = String::from_utf8_lossy(&axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap()).into_owned();
    assert!(html.contains("alert-error"));
    assert!(html.contains("Please select a customer."));
}

#[tokio::test]
async fn test_length_units_in_dim_option() {
    use axum::response::IntoResponse;
    use kds_tagore_rs::work_orders::{
        entities::draft_work_order_line,
        handlers,
        migrations::Migrator,
        seed::ensure_standard_seeds,
        state::WorkOrdersState,
    };
    use lariv_rs::{
        components::SharedChromeFolder, http::Cap,
        plugins::users::middleware::OptionalAuth,
    };
    use rust_decimal::Decimal;
    use sea_orm_migration::MigratorTrait;

    let db = sea_orm::Database::connect("sqlite::memory:").await.expect("sqlite db");
    Migrator::up(&db, None).await.expect("migrations up");
    ensure_standard_seeds(&db).await.expect("seeds ok");

    let state = WorkOrdersState { db: db.clone() };
    struct DummyFolder;
    impl lariv_rs::components::FoldChrome for DummyFolder {
        fn fold(&self, _ctx: &lariv_rs::components::SlotCtx) -> lariv_rs::components::ShellChrome {
            lariv_rs::components::ShellChrome::default()
        }
    }
    let chrome: SharedChromeFolder = std::sync::Arc::new(DummyFolder);

    // 1. Verify Draft Work Order create form renders the unit selector with all required units
    let res = handlers::work_order_create_get(
        Cap(state.clone()),
        Cap(chrome.clone()),
        OptionalAuth(None),
        axum::extract::Query(lariv_rs::web::ModalFormQuery::default()),
    ).await.into_response();

    assert_eq!(res.status(), axum::http::StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let html = String::from_utf8_lossy(&body);

    for unit in ["mm", "cm", "m", "km", "in", "ft"] {
        assert!(
            html.contains(&format!("<option value=\"{unit}\">{unit}</option>")),
            "Form must contain length unit option for '{unit}'"
        );
    }
    assert!(html.contains("unitFactors"), "Form script must define unitFactors for conversions");

    // 2. Test format_variables_display on draft_work_order_line Model
    let line_mm = draft_work_order_line::Model {
        id: 1,
        created_at: None,
        updated_at: None,
        draft_work_order_id: 1,
        component_id: 1,
        variables: serde_json::json!({ "length": 1000.0 }),
        quantity: Decimal::ONE,
        unit_weight: Decimal::ONE,
        material_rate: Decimal::from(100),
        final_cost: Decimal::from(100),
        extra_data: serde_json::json!({ "dim_unit": "mm" }),
    };
    assert_eq!(line_mm.format_variables_display(), "length: 1000 mm");

    let line_in = draft_work_order_line::Model {
        id: 2,
        created_at: None,
        updated_at: None,
        draft_work_order_id: 1,
        component_id: 1,
        variables: serde_json::json!({ "length": 254.0 }),
        quantity: Decimal::ONE,
        unit_weight: Decimal::ONE,
        material_rate: Decimal::from(100),
        final_cost: Decimal::from(100),
        extra_data: serde_json::json!({ "dim_unit": "in" }),
    };
    assert_eq!(line_in.format_variables_display(), "length: 10 in (254 mm)");

    let line_m = draft_work_order_line::Model {
        id: 3,
        created_at: None,
        updated_at: None,
        draft_work_order_id: 1,
        component_id: 1,
        variables: serde_json::json!({ "length": 2500.0 }),
        quantity: Decimal::ONE,
        unit_weight: Decimal::ONE,
        material_rate: Decimal::from(100),
        final_cost: Decimal::from(100),
        extra_data: serde_json::json!({ "dim_unit": "m" }),
    };
    assert_eq!(line_m.format_variables_display(), "length: 2.5 m (2500 mm)");

    // 3. Test multi-variable line with different per-variable units
    let line_multi = draft_work_order_line::Model {
        id: 4,
        created_at: None,
        updated_at: None,
        draft_work_order_id: 1,
        component_id: 1,
        variables: serde_json::json!({ "length": 304.8, "width": 50.0 }),
        quantity: Decimal::ONE,
        unit_weight: Decimal::ONE,
        material_rate: Decimal::from(100),
        final_cost: Decimal::from(100),
        extra_data: serde_json::json!({ "dim_units": { "length": "ft", "width": "mm" } }),
    };
    assert_eq!(line_multi.format_variables_display(), "length: 1 ft (304.8 mm), width: 50 mm");
}

#[tokio::test]
async fn test_draft_work_order_material_lines_rename() {
    use kds_tagore_rs::work_orders::{
        entities::{draft_work_order_material_line, draft_work_order_line, work_order_line},
        migrations::Migrator,
    };
    use sea_orm::EntityTrait;
    use sea_orm_migration::MigratorTrait;

    let db = sea_orm::Database::connect("sqlite::memory:").await.expect("sqlite db");
    Migrator::up(&db, None).await.expect("migrations up");

    // draft_work_order_material_lines table is queryable
    let material_lines = draft_work_order_material_line::Entity::find().all(&db).await.expect("find material lines");
    assert_eq!(material_lines.len(), 0);

    // backward compatibility aliases work
    let legacy_lines = draft_work_order_line::Entity::find().all(&db).await.expect("find draft lines alias");
    assert_eq!(legacy_lines.len(), 0);

    let wo_lines = work_order_line::Entity::find().all(&db).await.expect("find wo lines alias");
    assert_eq!(wo_lines.len(), 0);
}
