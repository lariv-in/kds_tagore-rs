//! Save and load a work order plan.

use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DatabaseConnection,
    EntityTrait, QueryFilter, TransactionTrait,
};

use crate::entities::{
    CompletedStepEntity, CompletedStepInputEntity, CompletedStepOutputEntity, PartEntity,
    PresuppliedPartEntity, StepEntity, StepInputEntity, StepOutputEntity, WorkOrderEntity,
    completed_step, completed_step_input, completed_step_output, part, presupplied_part, step,
    step_input, step_output, work_order,
};
use crate::graph::{self, GraphFlow, GraphPart, GraphStep, GraphSupply, PlanGraph};
use crate::measure;
use crate::plan_lines::PlanDraft;

fn map_db(err: sea_orm::DbErr) -> String {
    let text = err.to_string();
    if text.to_ascii_lowercase().contains("unique") {
        "That name or part is already used on this work order.".to_string()
    } else {
        text
    }
}

fn require_name(name: &str) -> Result<String, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        Err("Enter a name.".to_string())
    } else {
        Ok(trimmed.to_string())
    }
}

fn same_name(left: &str, right: &str) -> bool {
    left.trim().eq_ignore_ascii_case(right.trim())
}

pub struct NewWorkOrder {
    pub title: String,
    pub customer_id: i64,
    pub folder_vnode_id: i64,
}

pub struct PartInput {
    pub work_order_id: i64,
    pub existing_id: Option<i64>,
    pub name: String,
    pub measure_kind: String,
    pub measure_unit: String,
}

pub struct PresuppliedInput {
    pub work_order_id: i64,
    pub existing_id: Option<i64>,
    pub part_id: i64,
    pub qty_value: String,
    pub qty_unit: String,
}

pub struct StepInput {
    pub work_order_id: i64,
    pub existing_id: Option<i64>,
    pub name: String,
    pub planned_batches: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlowSide {
    Input,
    Output,
}

pub struct FlowInput {
    pub step_id: i64,
    pub existing_id: Option<i64>,
    pub part_id: i64,
    pub qty_value: String,
    pub qty_unit: String,
    pub side: FlowSide,
}

pub struct ActualLine {
    pub part_id: i64,
    pub qty_value: Decimal,
    pub qty_unit: String,
}

pub struct Plan {
    pub order: work_order::Model,
    pub parts: Vec<part::Model>,
    pub presupplied: Vec<presupplied_part::Model>,
    pub steps: Vec<step::Model>,
    pub inputs: Vec<step_input::Model>,
    pub outputs: Vec<step_output::Model>,
    pub completions: Vec<completed_step::Model>,
    pub completion_inputs: Vec<completed_step_input::Model>,
    pub completion_outputs: Vec<completed_step_output::Model>,
}

pub async fn create_work_order(
    db: &DatabaseConnection,
    input: NewWorkOrder,
) -> Result<work_order::Model, String> {
    let title = require_name(&input.title)?;
    if input.customer_id <= 0 {
        return Err("Select a customer.".to_string());
    }
    if input.folder_vnode_id <= 0 {
        return Err("Select a file or folder.".to_string());
    }
    let now = Utc::now();
    work_order::ActiveModel {
        title: Set(title),
        customer_id: Set(input.customer_id),
        folder_vnode_id: Set(input.folder_vnode_id),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(map_db)
}

pub async fn update_work_order(
    db: &DatabaseConnection,
    id: i64,
    title: &str,
    customer_id: i64,
    folder_vnode_id: i64,
) -> Result<work_order::Model, String> {
    let title = require_name(title)?;
    if customer_id <= 0 {
        return Err("Select a customer.".to_string());
    }
    if folder_vnode_id <= 0 {
        return Err("Select a file or folder.".to_string());
    }
    let existing = find_order(db, id).await?;
    let mut active: work_order::ActiveModel = existing.into();
    active.title = Set(title);
    active.customer_id = Set(customer_id);
    active.folder_vnode_id = Set(folder_vnode_id);
    active.updated_at = Set(Some(Utc::now()));
    active.update(db).await.map_err(map_db)
}

pub async fn delete_work_order(db: &DatabaseConnection, id: i64) -> Result<(), String> {
    let plan = load_plan(db, id).await?;
    if !plan.completions.is_empty() {
        return Err("Delete recorded batches before deleting this work order.".to_string());
    }
    let txn = db.begin().await.map_err(map_db)?;
    let step_ids: Vec<i64> = plan.steps.iter().map(|step| step.id).collect();
    if !step_ids.is_empty() {
        StepInputEntity::delete_many()
            .filter(step_input::Column::StepId.is_in(step_ids.clone()))
            .exec(&txn)
            .await
            .map_err(map_db)?;
        StepOutputEntity::delete_many()
            .filter(step_output::Column::StepId.is_in(step_ids))
            .exec(&txn)
            .await
            .map_err(map_db)?;
    }
    StepEntity::delete_many()
        .filter(step::Column::WorkOrderId.eq(id))
        .exec(&txn)
        .await
        .map_err(map_db)?;
    PresuppliedPartEntity::delete_many()
        .filter(presupplied_part::Column::WorkOrderId.eq(id))
        .exec(&txn)
        .await
        .map_err(map_db)?;
    PartEntity::delete_many()
        .filter(part::Column::WorkOrderId.eq(id))
        .exec(&txn)
        .await
        .map_err(map_db)?;
    work_order::ActiveModel {
        id: Set(id),
        ..Default::default()
    }
    .delete(&txn)
    .await
    .map_err(map_db)?;
    txn.commit().await.map_err(map_db)
}

pub async fn save_part(db: &DatabaseConnection, input: PartInput) -> Result<part::Model, String> {
    let name = require_name(&input.name)?;
    let (kind, unit) = measure::part_measure(&input.measure_kind, &input.measure_unit)?;
    find_order(db, input.work_order_id).await?;
    let parts = parts_of(db, input.work_order_id).await?;
    if parts
        .iter()
        .any(|row| input.existing_id != Some(row.id) && same_name(&row.name, &name))
    {
        return Err("A part with this name is already on the work order.".to_string());
    }
    if let Some(id) = input.existing_id {
        recheck_part_lines(db, id, kind, &unit).await?;
    }
    let now = Utc::now();
    if let Some(id) = input.existing_id {
        let existing = find_part(db, id).await?;
        if existing.work_order_id != input.work_order_id {
            return Err("That part is on a different work order.".to_string());
        }
        let mut active: part::ActiveModel = existing.into();
        active.name = Set(name);
        active.measure_kind = Set(kind.as_str().to_string());
        active.measure_unit = Set(unit);
        active.updated_at = Set(Some(now));
        active.update(db).await.map_err(map_db)
    } else {
        part::ActiveModel {
            work_order_id: Set(input.work_order_id),
            name: Set(name),
            measure_kind: Set(kind.as_str().to_string()),
            measure_unit: Set(unit),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(map_db)
    }
}

pub async fn delete_part(db: &DatabaseConnection, id: i64) -> Result<i64, String> {
    let existing = find_part(db, id).await?;
    if part_is_used(db, id).await? {
        return Err(
            "This part is still used by a step, a supply line, or a recorded batch.".to_string(),
        );
    }
    part::ActiveModel {
        id: Set(id),
        ..Default::default()
    }
    .delete(db)
    .await
    .map_err(map_db)?;
    Ok(existing.work_order_id)
}

pub async fn save_presupplied(
    db: &DatabaseConnection,
    input: PresuppliedInput,
) -> Result<presupplied_part::Model, String> {
    let part = part_on_order(db, input.work_order_id, input.part_id).await?;
    let kind = measure::parse_kind(&part.measure_kind)?;
    let (qty, unit) =
        measure::parse_line(kind, &part.measure_unit, &input.qty_value, &input.qty_unit)?;
    let rows = PresuppliedPartEntity::find()
        .filter(presupplied_part::Column::WorkOrderId.eq(input.work_order_id))
        .all(db)
        .await
        .map_err(map_db)?;
    if rows
        .iter()
        .any(|row| input.existing_id != Some(row.id) && row.part_id == input.part_id)
    {
        return Err("That part is already supplied.".to_string());
    }
    let now = Utc::now();
    if let Some(id) = input.existing_id {
        let existing = find_presupplied(db, id).await?;
        if existing.work_order_id != input.work_order_id {
            return Err("That supply line is on a different work order.".to_string());
        }
        let mut active: presupplied_part::ActiveModel = existing.into();
        active.part_id = Set(input.part_id);
        active.qty_value = Set(qty);
        active.qty_unit = Set(unit);
        active.updated_at = Set(Some(now));
        active.update(db).await.map_err(map_db)
    } else {
        presupplied_part::ActiveModel {
            work_order_id: Set(input.work_order_id),
            part_id: Set(input.part_id),
            qty_value: Set(qty),
            qty_unit: Set(unit),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(map_db)
    }
}

pub async fn delete_presupplied(db: &DatabaseConnection, id: i64) -> Result<i64, String> {
    let existing = find_presupplied(db, id).await?;
    presupplied_part::ActiveModel {
        id: Set(id),
        ..Default::default()
    }
    .delete(db)
    .await
    .map_err(map_db)?;
    Ok(existing.work_order_id)
}

pub async fn save_step(db: &DatabaseConnection, input: StepInput) -> Result<step::Model, String> {
    let name = require_name(&input.name)?;
    if input.planned_batches <= 0 {
        return Err("Planned batches must be at least 1.".to_string());
    }
    find_order(db, input.work_order_id).await?;
    let steps = steps_of(db, input.work_order_id).await?;
    if steps
        .iter()
        .any(|row| input.existing_id != Some(row.id) && same_name(&row.name, &name))
    {
        return Err("A step with this name is already on the work order.".to_string());
    }
    let now = Utc::now();
    if let Some(id) = input.existing_id {
        let existing = find_step(db, id).await?;
        if existing.work_order_id != input.work_order_id {
            return Err("That step is on a different work order.".to_string());
        }
        let mut active: step::ActiveModel = existing.into();
        active.name = Set(name);
        active.planned_batches = Set(input.planned_batches);
        active.updated_at = Set(Some(now));
        active.update(db).await.map_err(map_db)
    } else {
        step::ActiveModel {
            work_order_id: Set(input.work_order_id),
            name: Set(name),
            planned_batches: Set(input.planned_batches),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(map_db)
    }
}

pub async fn delete_step(db: &DatabaseConnection, id: i64) -> Result<i64, String> {
    let existing = find_step(db, id).await?;
    let completions = CompletedStepEntity::find()
        .filter(completed_step::Column::StepId.eq(id))
        .all(db)
        .await
        .map_err(map_db)?;
    if !completions.is_empty() {
        return Err("Delete recorded batches before deleting this step.".to_string());
    }
    let txn = db.begin().await.map_err(map_db)?;
    StepInputEntity::delete_many()
        .filter(step_input::Column::StepId.eq(id))
        .exec(&txn)
        .await
        .map_err(map_db)?;
    StepOutputEntity::delete_many()
        .filter(step_output::Column::StepId.eq(id))
        .exec(&txn)
        .await
        .map_err(map_db)?;
    step::ActiveModel {
        id: Set(id),
        ..Default::default()
    }
    .delete(&txn)
    .await
    .map_err(map_db)?;
    txn.commit().await.map_err(map_db)?;
    Ok(existing.work_order_id)
}

pub async fn save_flow(db: &DatabaseConnection, input: FlowInput) -> Result<i64, String> {
    let step = find_step(db, input.step_id).await?;
    let part = part_on_order(db, step.work_order_id, input.part_id).await?;
    let kind = measure::parse_kind(&part.measure_kind)?;
    let (qty, unit) =
        measure::parse_line(kind, &part.measure_unit, &input.qty_value, &input.qty_unit)?;
    let qty_in_part_unit = measure::in_part_unit(kind, &part.measure_unit, qty, &unit)?;
    let inputs = inputs_of_order(db, step.work_order_id).await?;
    let outputs = outputs_of_order(db, step.work_order_id).await?;
    if opposite_uses_part(&inputs, &outputs, input.step_id, input.part_id, input.side) {
        return Err("A step cannot consume and produce the same part.".to_string());
    }
    if same_side_uses_part(
        &inputs,
        &outputs,
        input.step_id,
        input.part_id,
        input.existing_id,
        input.side,
    ) {
        return Err("That part is already on this side of the step.".to_string());
    }
    let graph = graph_with_flow(
        db,
        step.work_order_id,
        &inputs,
        &outputs,
        &input,
        qty_in_part_unit,
    )
    .await?;
    if graph::layout(&graph).is_err() {
        return Err("This connection loops.".to_string());
    }
    let now = Utc::now();
    match input.side {
        FlowSide::Input => save_input_row(db, &input, qty, unit, now).await?,
        FlowSide::Output => save_output_row(db, &input, qty, unit, now).await?,
    }
    Ok(step.work_order_id)
}

pub async fn delete_flow(db: &DatabaseConnection, id: i64, side: FlowSide) -> Result<i64, String> {
    match side {
        FlowSide::Input => {
            let existing = StepInputEntity::find_by_id(id)
                .one(db)
                .await
                .map_err(map_db)?
                .ok_or_else(|| "That input line was not found.".to_string())?;
            let step = find_step(db, existing.step_id).await?;
            step_input::ActiveModel {
                id: Set(id),
                ..Default::default()
            }
            .delete(db)
            .await
            .map_err(map_db)?;
            Ok(step.work_order_id)
        }
        FlowSide::Output => {
            let existing = StepOutputEntity::find_by_id(id)
                .one(db)
                .await
                .map_err(map_db)?
                .ok_or_else(|| "That output line was not found.".to_string())?;
            let step = find_step(db, existing.step_id).await?;
            step_output::ActiveModel {
                id: Set(id),
                ..Default::default()
            }
            .delete(db)
            .await
            .map_err(map_db)?;
            Ok(step.work_order_id)
        }
    }
}

pub async fn complete_step(
    db: &DatabaseConnection,
    step_id: i64,
    user_id: i64,
    inputs: Vec<ActualLine>,
    outputs: Vec<ActualLine>,
) -> Result<completed_step::Model, String> {
    if user_id <= 0 {
        return Err("Sign in before recording a batch.".to_string());
    }
    let step = find_step(db, step_id).await?;
    let parts = parts_of(db, step.work_order_id).await?;
    validate_actuals(&parts, &inputs, &outputs)?;
    let now = Utc::now();
    let txn = db.begin().await.map_err(map_db)?;
    let saved = completed_step::ActiveModel {
        step_id: Set(step_id),
        completed_at: Set(now),
        completed_by: Set(user_id),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        ..Default::default()
    }
    .insert(&txn)
    .await
    .map_err(map_db)?;
    for line in &inputs {
        completed_step_input::ActiveModel {
            completed_step_id: Set(saved.id),
            part_id: Set(line.part_id),
            qty_value: Set(line.qty_value),
            qty_unit: Set(line.qty_unit.clone()),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            ..Default::default()
        }
        .insert(&txn)
        .await
        .map_err(map_db)?;
    }
    for line in &outputs {
        completed_step_output::ActiveModel {
            completed_step_id: Set(saved.id),
            part_id: Set(line.part_id),
            qty_value: Set(line.qty_value),
            qty_unit: Set(line.qty_unit.clone()),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            ..Default::default()
        }
        .insert(&txn)
        .await
        .map_err(map_db)?;
    }
    txn.commit().await.map_err(map_db)?;
    Ok(saved)
}

pub async fn delete_completion(db: &DatabaseConnection, id: i64) -> Result<i64, String> {
    let existing = CompletedStepEntity::find_by_id(id)
        .one(db)
        .await
        .map_err(map_db)?
        .ok_or_else(|| "That batch was not found.".to_string())?;
    let step = find_step(db, existing.step_id).await?;
    completed_step::ActiveModel {
        id: Set(id),
        ..Default::default()
    }
    .delete(db)
    .await
    .map_err(map_db)?;
    Ok(step.work_order_id)
}

pub async fn load_plan(db: &DatabaseConnection, id: i64) -> Result<Plan, String> {
    let order = find_order(db, id).await?;
    let parts = parts_of(db, id).await?;
    let presupplied = PresuppliedPartEntity::find()
        .filter(presupplied_part::Column::WorkOrderId.eq(id))
        .all(db)
        .await
        .map_err(map_db)?;
    let steps = steps_of(db, id).await?;
    let inputs = inputs_of_order(db, id).await?;
    let outputs = outputs_of_order(db, id).await?;
    let step_ids: Vec<i64> = steps.iter().map(|step| step.id).collect();
    let (completions, completion_inputs, completion_outputs) = if step_ids.is_empty() {
        (Vec::new(), Vec::new(), Vec::new())
    } else {
        let completions = CompletedStepEntity::find()
            .filter(completed_step::Column::StepId.is_in(step_ids))
            .all(db)
            .await
            .map_err(map_db)?;
        let completion_ids: Vec<i64> = completions.iter().map(|row| row.id).collect();
        if completion_ids.is_empty() {
            (completions, Vec::new(), Vec::new())
        } else {
            let completion_inputs = CompletedStepInputEntity::find()
                .filter(completed_step_input::Column::CompletedStepId.is_in(completion_ids.clone()))
                .all(db)
                .await
                .map_err(map_db)?;
            let completion_outputs = CompletedStepOutputEntity::find()
                .filter(completed_step_output::Column::CompletedStepId.is_in(completion_ids))
                .all(db)
                .await
                .map_err(map_db)?;
            (completions, completion_inputs, completion_outputs)
        }
    };
    Ok(Plan {
        order,
        parts,
        presupplied,
        steps,
        inputs,
        outputs,
        completions,
        completion_inputs,
        completion_outputs,
    })
}

pub fn plan_graph(plan: &Plan) -> Result<PlanGraph, String> {
    let mut inputs = Vec::new();
    for line in &plan.inputs {
        inputs.push(flow_from_line(
            &plan.parts,
            line.step_id,
            line.part_id,
            line.qty_value,
            &line.qty_unit,
        )?);
    }
    let mut outputs = Vec::new();
    for line in &plan.outputs {
        outputs.push(flow_from_line(
            &plan.parts,
            line.step_id,
            line.part_id,
            line.qty_value,
            &line.qty_unit,
        )?);
    }
    let mut presupplied = Vec::new();
    for line in &plan.presupplied {
        let part = plan
            .parts
            .iter()
            .find(|part| part.id == line.part_id)
            .ok_or_else(|| "A supply line points at a missing part.".to_string())?;
        let kind = measure::parse_kind(&part.measure_kind)?;
        let qty = measure::in_part_unit(kind, &part.measure_unit, line.qty_value, &line.qty_unit)?;
        presupplied.push(GraphSupply {
            part_id: line.part_id,
            qty,
        });
    }
    Ok(PlanGraph {
        parts: plan
            .parts
            .iter()
            .map(|part| GraphPart {
                id: part.id,
                name: part.name.clone(),
            })
            .collect(),
        steps: plan
            .steps
            .iter()
            .map(|step| GraphStep {
                id: step.id,
                name: step.name.clone(),
                planned_batches: step.planned_batches,
            })
            .collect(),
        inputs,
        outputs,
        presupplied,
    })
}

async fn find_order<C: ConnectionTrait>(db: &C, id: i64) -> Result<work_order::Model, String> {
    WorkOrderEntity::find_by_id(id)
        .one(db)
        .await
        .map_err(map_db)?
        .ok_or_else(|| "That work order was not found.".to_string())
}

async fn find_part<C: ConnectionTrait>(db: &C, id: i64) -> Result<part::Model, String> {
    PartEntity::find_by_id(id)
        .one(db)
        .await
        .map_err(map_db)?
        .ok_or_else(|| "That part was not found.".to_string())
}

async fn find_step<C: ConnectionTrait>(db: &C, id: i64) -> Result<step::Model, String> {
    StepEntity::find_by_id(id)
        .one(db)
        .await
        .map_err(map_db)?
        .ok_or_else(|| "That step was not found.".to_string())
}

async fn find_presupplied<C: ConnectionTrait>(
    db: &C,
    id: i64,
) -> Result<presupplied_part::Model, String> {
    PresuppliedPartEntity::find_by_id(id)
        .one(db)
        .await
        .map_err(map_db)?
        .ok_or_else(|| "That supply line was not found.".to_string())
}

async fn parts_of<C: ConnectionTrait>(
    db: &C,
    work_order_id: i64,
) -> Result<Vec<part::Model>, String> {
    PartEntity::find()
        .filter(part::Column::WorkOrderId.eq(work_order_id))
        .all(db)
        .await
        .map_err(map_db)
}

async fn steps_of<C: ConnectionTrait>(
    db: &C,
    work_order_id: i64,
) -> Result<Vec<step::Model>, String> {
    StepEntity::find()
        .filter(step::Column::WorkOrderId.eq(work_order_id))
        .all(db)
        .await
        .map_err(map_db)
}

async fn part_on_order<C: ConnectionTrait>(
    db: &C,
    work_order_id: i64,
    part_id: i64,
) -> Result<part::Model, String> {
    let part = find_part(db, part_id).await?;
    if part.work_order_id != work_order_id {
        return Err("Choose a part from this work order.".to_string());
    }
    Ok(part)
}

async fn inputs_of_order<C: ConnectionTrait>(
    db: &C,
    work_order_id: i64,
) -> Result<Vec<step_input::Model>, String> {
    let steps = steps_of(db, work_order_id).await?;
    let ids: Vec<i64> = steps.iter().map(|step| step.id).collect();
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    StepInputEntity::find()
        .filter(step_input::Column::StepId.is_in(ids))
        .all(db)
        .await
        .map_err(map_db)
}

async fn outputs_of_order<C: ConnectionTrait>(
    db: &C,
    work_order_id: i64,
) -> Result<Vec<step_output::Model>, String> {
    let steps = steps_of(db, work_order_id).await?;
    let ids: Vec<i64> = steps.iter().map(|step| step.id).collect();
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    StepOutputEntity::find()
        .filter(step_output::Column::StepId.is_in(ids))
        .all(db)
        .await
        .map_err(map_db)
}

async fn part_is_used<C: ConnectionTrait>(db: &C, part_id: i64) -> Result<bool, String> {
    let input = StepInputEntity::find()
        .filter(step_input::Column::PartId.eq(part_id))
        .one(db)
        .await
        .map_err(map_db)?;
    let output = StepOutputEntity::find()
        .filter(step_output::Column::PartId.eq(part_id))
        .one(db)
        .await
        .map_err(map_db)?;
    let supplied = PresuppliedPartEntity::find()
        .filter(presupplied_part::Column::PartId.eq(part_id))
        .one(db)
        .await
        .map_err(map_db)?;
    let done_in = CompletedStepInputEntity::find()
        .filter(completed_step_input::Column::PartId.eq(part_id))
        .one(db)
        .await
        .map_err(map_db)?;
    let done_out = CompletedStepOutputEntity::find()
        .filter(completed_step_output::Column::PartId.eq(part_id))
        .one(db)
        .await
        .map_err(map_db)?;
    Ok(input.is_some()
        || output.is_some()
        || supplied.is_some()
        || done_in.is_some()
        || done_out.is_some())
}

async fn recheck_part_lines<C: ConnectionTrait>(
    db: &C,
    part_id: i64,
    kind: lariv_formula::VariableType,
    unit: &str,
) -> Result<(), String> {
    let inputs = StepInputEntity::find()
        .filter(step_input::Column::PartId.eq(part_id))
        .all(db)
        .await
        .map_err(map_db)?;
    for line in inputs {
        measure::in_part_unit(kind, unit, line.qty_value, &line.qty_unit)?;
    }
    let outputs = StepOutputEntity::find()
        .filter(step_output::Column::PartId.eq(part_id))
        .all(db)
        .await
        .map_err(map_db)?;
    for line in outputs {
        measure::in_part_unit(kind, unit, line.qty_value, &line.qty_unit)?;
    }
    let supplied = PresuppliedPartEntity::find()
        .filter(presupplied_part::Column::PartId.eq(part_id))
        .all(db)
        .await
        .map_err(map_db)?;
    for line in supplied {
        measure::in_part_unit(kind, unit, line.qty_value, &line.qty_unit)?;
    }
    Ok(())
}

fn opposite_uses_part(
    inputs: &[step_input::Model],
    outputs: &[step_output::Model],
    step_id: i64,
    part_id: i64,
    side: FlowSide,
) -> bool {
    match side {
        FlowSide::Input => outputs
            .iter()
            .any(|line| line.step_id == step_id && line.part_id == part_id),
        FlowSide::Output => inputs
            .iter()
            .any(|line| line.step_id == step_id && line.part_id == part_id),
    }
}

fn same_side_uses_part(
    inputs: &[step_input::Model],
    outputs: &[step_output::Model],
    step_id: i64,
    part_id: i64,
    existing_id: Option<i64>,
    side: FlowSide,
) -> bool {
    match side {
        FlowSide::Input => inputs.iter().any(|line| {
            line.step_id == step_id && line.part_id == part_id && existing_id != Some(line.id)
        }),
        FlowSide::Output => outputs.iter().any(|line| {
            line.step_id == step_id && line.part_id == part_id && existing_id != Some(line.id)
        }),
    }
}

async fn graph_with_flow<C: ConnectionTrait>(
    db: &C,
    work_order_id: i64,
    inputs: &[step_input::Model],
    outputs: &[step_output::Model],
    pending: &FlowInput,
    qty_in_part_unit: Decimal,
) -> Result<PlanGraph, String> {
    let parts = parts_of(db, work_order_id).await?;
    let steps = steps_of(db, work_order_id).await?;
    let presupplied = PresuppliedPartEntity::find()
        .filter(presupplied_part::Column::WorkOrderId.eq(work_order_id))
        .all(db)
        .await
        .map_err(map_db)?;
    let mut graph_inputs = Vec::new();
    for line in inputs {
        if pending.side == FlowSide::Input && pending.existing_id == Some(line.id) {
            continue;
        }
        graph_inputs.push(flow_from_line(
            &parts,
            line.step_id,
            line.part_id,
            line.qty_value,
            &line.qty_unit,
        )?);
    }
    let mut graph_outputs = Vec::new();
    for line in outputs {
        if pending.side == FlowSide::Output && pending.existing_id == Some(line.id) {
            continue;
        }
        graph_outputs.push(flow_from_line(
            &parts,
            line.step_id,
            line.part_id,
            line.qty_value,
            &line.qty_unit,
        )?);
    }
    let pending_flow = GraphFlow {
        step_id: pending.step_id,
        part_id: pending.part_id,
        qty: qty_in_part_unit,
    };
    match pending.side {
        FlowSide::Input => graph_inputs.push(pending_flow),
        FlowSide::Output => graph_outputs.push(pending_flow),
    }
    let mut supplies = Vec::new();
    for line in presupplied {
        let Some(part) = parts.iter().find(|part| part.id == line.part_id) else {
            continue;
        };
        let kind = measure::parse_kind(&part.measure_kind)?;
        supplies.push(GraphSupply {
            part_id: line.part_id,
            qty: measure::in_part_unit(kind, &part.measure_unit, line.qty_value, &line.qty_unit)?,
        });
    }
    Ok(PlanGraph {
        parts: parts
            .iter()
            .map(|part| GraphPart {
                id: part.id,
                name: part.name.clone(),
            })
            .collect(),
        steps: steps
            .iter()
            .map(|step| GraphStep {
                id: step.id,
                name: step.name.clone(),
                planned_batches: step.planned_batches,
            })
            .collect(),
        inputs: graph_inputs,
        outputs: graph_outputs,
        presupplied: supplies,
    })
}

fn flow_from_line(
    parts: &[part::Model],
    step_id: i64,
    part_id: i64,
    qty_value: Decimal,
    qty_unit: &str,
) -> Result<GraphFlow, String> {
    let part = parts
        .iter()
        .find(|part| part.id == part_id)
        .ok_or_else(|| "A step line points at a missing part.".to_string())?;
    let kind = measure::parse_kind(&part.measure_kind)?;
    Ok(GraphFlow {
        step_id,
        part_id,
        qty: measure::in_part_unit(kind, &part.measure_unit, qty_value, qty_unit)?,
    })
}

async fn save_input_row(
    db: &DatabaseConnection,
    input: &FlowInput,
    qty: Decimal,
    unit: String,
    now: chrono::DateTime<Utc>,
) -> Result<(), String> {
    if let Some(id) = input.existing_id {
        let existing = StepInputEntity::find_by_id(id)
            .one(db)
            .await
            .map_err(map_db)?
            .ok_or_else(|| "That input line was not found.".to_string())?;
        if existing.step_id != input.step_id {
            return Err("That input line is on a different step.".to_string());
        }
        let mut active: step_input::ActiveModel = existing.into();
        active.part_id = Set(input.part_id);
        active.qty_value = Set(qty);
        active.qty_unit = Set(unit);
        active.updated_at = Set(Some(now));
        active.update(db).await.map_err(map_db)?;
    } else {
        step_input::ActiveModel {
            step_id: Set(input.step_id),
            part_id: Set(input.part_id),
            qty_value: Set(qty),
            qty_unit: Set(unit),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(map_db)?;
    }
    Ok(())
}

async fn save_output_row(
    db: &DatabaseConnection,
    input: &FlowInput,
    qty: Decimal,
    unit: String,
    now: chrono::DateTime<Utc>,
) -> Result<(), String> {
    if let Some(id) = input.existing_id {
        let existing = StepOutputEntity::find_by_id(id)
            .one(db)
            .await
            .map_err(map_db)?
            .ok_or_else(|| "That output line was not found.".to_string())?;
        if existing.step_id != input.step_id {
            return Err("That output line is on a different step.".to_string());
        }
        let mut active: step_output::ActiveModel = existing.into();
        active.part_id = Set(input.part_id);
        active.qty_value = Set(qty);
        active.qty_unit = Set(unit);
        active.updated_at = Set(Some(now));
        active.update(db).await.map_err(map_db)?;
    } else {
        step_output::ActiveModel {
            step_id: Set(input.step_id),
            part_id: Set(input.part_id),
            qty_value: Set(qty),
            qty_unit: Set(unit),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(map_db)?;
    }
    Ok(())
}

fn validate_actuals(
    parts: &[part::Model],
    inputs: &[ActualLine],
    outputs: &[ActualLine],
) -> Result<(), String> {
    let mut seen_in = Vec::new();
    for line in inputs {
        if seen_in.contains(&line.part_id) {
            return Err("Each part can appear once in the batch inputs.".to_string());
        }
        seen_in.push(line.part_id);
        check_actual(parts, line)?;
    }
    let mut seen_out = Vec::new();
    for line in outputs {
        if seen_in.contains(&line.part_id) {
            return Err("A batch cannot consume and produce the same part.".to_string());
        }
        if seen_out.contains(&line.part_id) {
            return Err("Each part can appear once in the batch outputs.".to_string());
        }
        seen_out.push(line.part_id);
        check_actual(parts, line)?;
    }
    Ok(())
}

pub async fn replace_plan_lines(
    db: &DatabaseConnection,
    work_order_id: i64,
    draft: &PlanDraft,
) -> Result<(), String> {
    let plan = load_plan(db, work_order_id).await?;
    refuse_removed(&plan, draft)?;
    clear_recipe(db, &plan).await?;
    let part_ids = sync_parts(db, work_order_id, &plan, draft).await?;
    let step_ids = sync_steps(db, work_order_id, &plan, draft).await?;
    write_recipe(db, work_order_id, &part_ids, &step_ids, draft).await
}

fn refuse_removed(plan: &Plan, draft: &PlanDraft) -> Result<(), String> {
    for part in &plan.parts {
        if draft
            .parts
            .iter()
            .any(|line| same_name(&line.name, &part.name))
        {
            continue;
        }
        let used = plan
            .completion_inputs
            .iter()
            .any(|line| line.part_id == part.id)
            || plan
                .completion_outputs
                .iter()
                .any(|line| line.part_id == part.id);
        if used {
            return Err(format!(
                "Delete recorded batches before removing {}.",
                part.name
            ));
        }
    }
    for step in &plan.steps {
        if draft
            .steps
            .iter()
            .any(|line| same_name(&line.name, &step.name))
        {
            continue;
        }
        if plan.completions.iter().any(|row| row.step_id == step.id) {
            return Err(format!(
                "Delete recorded batches before removing {}.",
                step.name
            ));
        }
    }
    Ok(())
}

async fn clear_recipe(db: &DatabaseConnection, plan: &Plan) -> Result<(), String> {
    for line in &plan.inputs {
        delete_flow(db, line.id, FlowSide::Input).await?;
    }
    for line in &plan.outputs {
        delete_flow(db, line.id, FlowSide::Output).await?;
    }
    for line in &plan.presupplied {
        delete_presupplied(db, line.id).await?;
    }
    Ok(())
}

async fn sync_parts(
    db: &DatabaseConnection,
    work_order_id: i64,
    plan: &Plan,
    draft: &PlanDraft,
) -> Result<Vec<i64>, String> {
    let mut ids = Vec::with_capacity(draft.parts.len());
    for line in &draft.parts {
        let existing = plan
            .parts
            .iter()
            .find(|row| same_name(&row.name, &line.name))
            .map(|row| row.id);
        ids.push(existing);
    }
    for id in ids.iter().copied().flatten() {
        let row = plan
            .parts
            .iter()
            .find(|part| part.id == id)
            .ok_or_else(|| "That part was not found.".to_string())?;
        save_part(
            db,
            PartInput {
                work_order_id,
                existing_id: Some(id),
                name: temp_name(id),
                measure_kind: row.measure_kind.clone(),
                measure_unit: row.measure_unit.clone(),
            },
        )
        .await?;
    }
    for row in &plan.parts {
        if !ids.contains(&Some(row.id)) {
            delete_part(db, row.id).await?;
        }
    }
    let mut saved_ids = Vec::with_capacity(draft.parts.len());
    for (existing, line) in ids.into_iter().zip(draft.parts.iter()) {
        let saved = save_part(
            db,
            PartInput {
                work_order_id,
                existing_id: existing,
                name: line.name.clone(),
                measure_kind: line.measure_kind.clone(),
                measure_unit: line.measure_unit.clone(),
            },
        )
        .await?;
        saved_ids.push(saved.id);
    }
    Ok(saved_ids)
}

async fn sync_steps(
    db: &DatabaseConnection,
    work_order_id: i64,
    plan: &Plan,
    draft: &PlanDraft,
) -> Result<Vec<i64>, String> {
    let mut ids = Vec::with_capacity(draft.steps.len());
    for line in &draft.steps {
        let existing = plan
            .steps
            .iter()
            .find(|row| same_name(&row.name, &line.name))
            .map(|row| row.id);
        ids.push(existing);
    }
    for id in ids.iter().copied().flatten() {
        let row = plan
            .steps
            .iter()
            .find(|step| step.id == id)
            .ok_or_else(|| "That step was not found.".to_string())?;
        save_step(
            db,
            StepInput {
                work_order_id,
                existing_id: Some(id),
                name: temp_name(id),
                planned_batches: row.planned_batches,
            },
        )
        .await?;
    }
    for row in &plan.steps {
        if !ids.contains(&Some(row.id)) {
            delete_step(db, row.id).await?;
        }
    }
    let mut saved_ids = Vec::with_capacity(draft.steps.len());
    for (existing, line) in ids.into_iter().zip(draft.steps.iter()) {
        let saved = save_step(
            db,
            StepInput {
                work_order_id,
                existing_id: existing,
                name: line.name.clone(),
                planned_batches: line.planned_batches,
            },
        )
        .await?;
        saved_ids.push(saved.id);
    }
    Ok(saved_ids)
}

async fn write_recipe(
    db: &DatabaseConnection,
    work_order_id: i64,
    part_ids: &[i64],
    step_ids: &[i64],
    draft: &PlanDraft,
) -> Result<(), String> {
    for (step_index, step) in draft.steps.iter().enumerate() {
        let step_id = *step_ids
            .get(step_index)
            .ok_or_else(|| "That step was not found.".to_string())?;
        for flow in &step.inputs {
            save_named_flow(db, part_ids, draft, step_id, flow, FlowSide::Input).await?;
        }
        for flow in &step.outputs {
            save_named_flow(db, part_ids, draft, step_id, flow, FlowSide::Output).await?;
        }
    }
    for supply in &draft.presupplied {
        let part_id = id_for_name(part_ids, &draft.parts, &supply.part_name)?;
        save_presupplied(
            db,
            PresuppliedInput {
                work_order_id,
                existing_id: None,
                part_id,
                qty_value: supply.qty_value.clone(),
                qty_unit: supply.qty_unit.clone(),
            },
        )
        .await?;
    }
    Ok(())
}

async fn save_named_flow(
    db: &DatabaseConnection,
    part_ids: &[i64],
    draft: &PlanDraft,
    step_id: i64,
    flow: &crate::plan_lines::DraftFlow,
    side: FlowSide,
) -> Result<(), String> {
    let part_id = id_for_name(part_ids, &draft.parts, &flow.part_name)?;
    save_flow(
        db,
        FlowInput {
            step_id,
            existing_id: None,
            part_id,
            qty_value: flow.qty_value.clone(),
            qty_unit: flow.qty_unit.clone(),
            side,
        },
    )
    .await?;
    Ok(())
}

fn id_for_name(
    part_ids: &[i64],
    parts: &[crate::plan_lines::DraftPart],
    name: &str,
) -> Result<i64, String> {
    parts
        .iter()
        .zip(part_ids.iter())
        .find(|(part, _)| same_name(&part.name, name))
        .map(|(_, id)| *id)
        .ok_or_else(|| "Choose a part from this work order.".to_string())
}

fn temp_name(id: i64) -> String {
    format!("\u{1}{id}")
}

fn check_actual(parts: &[part::Model], line: &ActualLine) -> Result<(), String> {
    if line.qty_value.is_sign_negative() {
        return Err("Quantity cannot be negative.".to_string());
    }
    let part = parts
        .iter()
        .find(|part| part.id == line.part_id)
        .ok_or_else(|| "Choose a part from this work order.".to_string())?;
    let kind = measure::parse_kind(&part.measure_kind)?;
    measure::in_part_unit(kind, &part.measure_unit, line.qty_value, &line.qty_unit)?;
    Ok(())
}
