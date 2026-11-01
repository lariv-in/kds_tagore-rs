//! Process-plan graph ranks and the SQLite save path.

use kds_tagore_rs::work_orders::entities::{part, presupplied_part};
use kds_tagore_rs::work_orders::graph::{
    GraphFlow, GraphPart, GraphStep, GraphSupply, PartRole, PlanGraph, PlanLayout, rank_of, role_of,
};
use kds_tagore_rs::work_orders::logic::{
    self, ActualLine, FlowInput, FlowSide, NewWorkOrder, PartInput, PresuppliedInput, StepInput,
};
use kds_tagore_rs::work_orders::plan_lines;
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait};

fn part(id: i64, name: &str) -> GraphPart {
    GraphPart {
        id,
        name: name.to_string(),
    }
}

fn step(id: i64, name: &str, planned_batches: i32) -> GraphStep {
    GraphStep {
        id,
        name: name.to_string(),
        planned_batches,
    }
}

fn flow(step_id: i64, part_id: i64, qty: i64) -> GraphFlow {
    GraphFlow {
        step_id,
        part_id,
        qty: Decimal::from(qty),
    }
}

fn chain() -> PlanGraph {
    PlanGraph {
        parts: vec![
            part(1, "Plate"),
            part(2, "Blank"),
            part(3, "Bracket"),
            part(4, "Spare"),
        ],
        steps: vec![step(10, "Cut", 2), step(11, "Bend", 1)],
        inputs: vec![flow(10, 1, 1), flow(11, 2, 1)],
        outputs: vec![flow(10, 2, 1), flow(11, 3, 1)],
        presupplied: Vec::new(),
    }
}

fn laid(plan: PlanGraph) -> PlanLayout {
    kds_tagore_rs::work_orders::graph::layout(&plan).expect("plan ranks")
}

#[test]
fn graph_classifies_four_roles() {
    let layout = laid(chain());
    assert_eq!(role_of(&layout, 1), Some(PartRole::Raw));
    assert_eq!(role_of(&layout, 2), Some(PartRole::Intermediate));
    assert_eq!(role_of(&layout, 3), Some(PartRole::Demanded));
    assert_eq!(role_of(&layout, 4), Some(PartRole::Unused));
}

#[test]
fn graph_ranks_a_two_step_chain() {
    let layout = laid(chain());
    assert_eq!(rank_of(&layout.part_ranks, 1), Some(0));
    assert_eq!(rank_of(&layout.step_ranks, 10), Some(1));
    assert_eq!(rank_of(&layout.part_ranks, 2), Some(2));
    assert_eq!(rank_of(&layout.step_ranks, 11), Some(3));
    assert_eq!(rank_of(&layout.part_ranks, 3), Some(4));
}

#[test]
fn graph_rejects_a_cycle() {
    let plan = PlanGraph {
        parts: vec![part(1, "A"), part(2, "B")],
        steps: vec![step(10, "One", 1), step(11, "Two", 1)],
        inputs: vec![flow(10, 1, 1), flow(11, 2, 1)],
        outputs: vec![flow(10, 2, 1), flow(11, 1, 1)],
        presupplied: Vec::new(),
    };
    assert!(kds_tagore_rs::work_orders::graph::layout(&plan).is_err());
}

#[test]
fn graph_co_product_shares_a_rank() {
    let plan = PlanGraph {
        parts: vec![part(1, "A"), part(2, "B"), part(3, "C"), part(4, "D")],
        steps: vec![step(10, "Split", 1), step(11, "Finish", 1)],
        inputs: vec![flow(10, 1, 1), flow(11, 2, 1)],
        outputs: vec![flow(10, 2, 1), flow(10, 3, 1), flow(11, 4, 1)],
        presupplied: Vec::new(),
    };
    let layout = laid(plan);
    assert_eq!(role_of(&layout, 2), Some(PartRole::Intermediate));
    assert_eq!(role_of(&layout, 3), Some(PartRole::Demanded));
    assert_eq!(
        rank_of(&layout.part_ranks, 2),
        rank_of(&layout.part_ranks, 3)
    );
    let b = rank_of(&layout.part_ranks, 2).expect("rank of B");
    let d = rank_of(&layout.part_ranks, 4).expect("rank of D");
    assert!(d > b);
}

#[test]
fn graph_coverage_includes_presupplied_gap() {
    let plan = PlanGraph {
        parts: vec![part(1, "Plate"), part(2, "Bracket")],
        steps: vec![step(10, "Cut", 2)],
        inputs: vec![flow(10, 1, 4)],
        outputs: vec![flow(10, 2, 1)],
        presupplied: vec![GraphSupply {
            part_id: 1,
            qty: Decimal::from(5),
        }],
    };
    let layout = laid(plan);
    let plate = layout
        .coverage
        .iter()
        .find(|row| row.part_id == 1)
        .expect("plate coverage");
    assert_eq!(plate.supply, Decimal::from(5));
    assert_eq!(plate.demand, Decimal::from(8));
    assert_eq!(plate.gap, Decimal::from(-3));
    let bracket = layout
        .coverage
        .iter()
        .find(|row| row.part_id == 2)
        .expect("bracket coverage");
    assert_eq!(bracket.supply, Decimal::from(2));
    assert_eq!(bracket.demand, Decimal::ZERO);
    assert_eq!(bracket.gap, Decimal::from(2));
}

async fn memory_db() -> sea_orm::DatabaseConnection {
    use sea_orm_migration::MigratorTrait;
    let db = sea_orm::Database::connect("sqlite::memory:")
        .await
        .expect("sqlite db");
    kds_tagore_rs::work_orders::migrations::Migrator::up(&db, None)
        .await
        .expect("work order migrations");
    db
}

#[tokio::test]
async fn sqlite_saves_a_plan_and_enforces_unique_keys() {
    let db = memory_db().await;
    let order = logic::create_work_order(
        &db,
        NewWorkOrder {
            title: "Bracket job".into(),
            customer_id: 1,
            folder_vnode_id: 1,
        },
    )
    .await
    .expect("create work order");
    let plate = logic::save_part(
        &db,
        PartInput {
            work_order_id: order.id,
            existing_id: None,
            name: "Plate".into(),
            measure_kind: "quantity".into(),
            measure_unit: "Number".into(),
        },
    )
    .await
    .expect("plate");
    let bracket = logic::save_part(
        &db,
        PartInput {
            work_order_id: order.id,
            existing_id: None,
            name: "Bracket".into(),
            measure_kind: "quantity".into(),
            measure_unit: "Number".into(),
        },
    )
    .await
    .expect("bracket");
    let cut = logic::save_step(
        &db,
        StepInput {
            work_order_id: order.id,
            existing_id: None,
            name: "Cut".into(),
            planned_batches: 2,
        },
    )
    .await
    .expect("step");
    logic::save_flow(
        &db,
        FlowInput {
            step_id: cut.id,
            existing_id: None,
            part_id: plate.id,
            qty_value: "1".into(),
            qty_unit: "Number".into(),
            side: FlowSide::Input,
        },
    )
    .await
    .expect("input");
    logic::save_flow(
        &db,
        FlowInput {
            step_id: cut.id,
            existing_id: None,
            part_id: bracket.id,
            qty_value: "1".into(),
            qty_unit: "Number".into(),
            side: FlowSide::Output,
        },
    )
    .await
    .expect("output");
    let done = logic::complete_step(
        &db,
        cut.id,
        1,
        vec![ActualLine {
            part_id: plate.id,
            qty_value: Decimal::from(1),
            qty_unit: "Number".into(),
        }],
        vec![ActualLine {
            part_id: bracket.id,
            qty_value: Decimal::from(1),
            qty_unit: "Number".into(),
        }],
    )
    .await
    .expect("completion");
    assert!(done.id > 0);

    let duplicate_part = part::ActiveModel {
        work_order_id: Set(order.id),
        name: Set("Plate".into()),
        measure_kind: Set("quantity".into()),
        measure_unit: Set("Number".into()),
        ..Default::default()
    }
    .insert(&db)
    .await;
    assert!(duplicate_part.is_err());

    logic::save_presupplied(
        &db,
        PresuppliedInput {
            work_order_id: order.id,
            existing_id: None,
            part_id: plate.id,
            qty_value: "5".into(),
            qty_unit: "Number".into(),
        },
    )
    .await
    .expect("presupplied");
    let duplicate_supply = presupplied_part::ActiveModel {
        work_order_id: Set(order.id),
        part_id: Set(plate.id),
        qty_value: Set(Decimal::ONE),
        qty_unit: Set("Number".into()),
        ..Default::default()
    }
    .insert(&db)
    .await;
    assert!(duplicate_supply.is_err());

    let weld = logic::save_step(
        &db,
        StepInput {
            work_order_id: order.id,
            existing_id: None,
            name: "Weld".into(),
            planned_batches: 1,
        },
    )
    .await
    .expect("second step");
    logic::save_flow(
        &db,
        FlowInput {
            step_id: weld.id,
            existing_id: None,
            part_id: bracket.id,
            qty_value: "1".into(),
            qty_unit: "Number".into(),
            side: FlowSide::Input,
        },
    )
    .await
    .expect("cycle input");
    let cycle = logic::save_flow(
        &db,
        FlowInput {
            step_id: weld.id,
            existing_id: None,
            part_id: plate.id,
            qty_value: "1".into(),
            qty_unit: "Number".into(),
            side: FlowSide::Output,
        },
    )
    .await;
    assert!(cycle.is_err());

    assert!(logic::delete_work_order(&db, order.id).await.is_err());
    logic::delete_completion(&db, done.id)
        .await
        .expect("delete batch");
    logic::delete_work_order(&db, order.id)
        .await
        .expect("delete work order");
    assert!(
        part::Entity::find_by_id(plate.id)
            .one(&db)
            .await
            .expect("lookup")
            .is_none()
    );
}

#[tokio::test]
async fn sqlite_form_lines_replace_the_plan() {
    let db = memory_db().await;
    let order = logic::create_work_order(
        &db,
        NewWorkOrder {
            title: "Bracket job".into(),
            customer_id: 1,
            folder_vnode_id: 1,
        },
    )
    .await
    .expect("create work order");
    let draft = plan_lines::parse_plan(
        r#"[{"name":"Plate","measure_kind":"quantity","measure_unit":"Number"},{"name":"Bracket","measure_kind":"quantity","measure_unit":"Number"}]"#,
        r#"[{"part_name":"Plate","qty_value":"5","qty_unit":"Number"}]"#,
        r#"[{"name":"Cut","planned_batches":"2","inputs":[{"part_name":"Plate","qty_value":"1","qty_unit":"Number"}],"outputs":[{"part_name":"Bracket","qty_value":"1","qty_unit":"Number"}]}]"#,
    )
    .expect("parse plan");
    logic::replace_plan_lines(&db, order.id, &draft)
        .await
        .expect("save plan");
    let plan = logic::load_plan(&db, order.id).await.expect("load plan");
    assert_eq!(plan.parts.len(), 2);
    assert_eq!(plan.presupplied.len(), 1);
    assert_eq!(plan.steps.len(), 1);
    assert_eq!(plan.steps[0].planned_batches, 2);
    assert_eq!(plan.inputs.len(), 1);
    assert_eq!(plan.outputs.len(), 1);
    let encoded = plan_lines::encode_plan(
        &plan.parts,
        &plan.presupplied,
        &plan.steps,
        &plan.inputs,
        &plan.outputs,
    );
    plan_lines::parse_plan(
        &encoded.parts_json,
        &encoded.presupplied_json,
        &encoded.steps_json,
    )
    .expect("round trip");
    assert!(
        plan_lines::parse_plan(
            r#"[{"name":"Plate","measure_kind":"quantity","measure_unit":"Number"},{"name":"Bracket","measure_kind":"quantity","measure_unit":"Number"}]"#,
            "[]",
            r#"[{"name":"Cut","planned_batches":"1","inputs":[{"part_name":"Plate","qty_value":"1","qty_unit":"Number"}],"outputs":[{"part_name":"Bracket","qty_value":"1","qty_unit":"Number"}]},{"name":"Weld","planned_batches":"1","inputs":[{"part_name":"Bracket","qty_value":"1","qty_unit":"Number"}],"outputs":[{"part_name":"Plate","qty_value":"1","qty_unit":"Number"}]}]"#,
        )
        .is_err()
    );
    let cleared = plan_lines::parse_plan("[]", "[]", "[]").expect("empty plan");
    logic::replace_plan_lines(&db, order.id, &cleared)
        .await
        .expect("clear plan");
    let plan = logic::load_plan(&db, order.id).await.expect("load cleared");
    assert!(plan.parts.is_empty());
    assert!(plan.steps.is_empty());
}
