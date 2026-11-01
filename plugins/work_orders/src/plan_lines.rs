//! Part, presupplied, and step lines posted with the work order form.

use rust_decimal::Decimal;
use serde::Deserialize;

use crate::entities::{part, presupplied_part, step, step_input, step_output};
use crate::graph::{self, GraphFlow, GraphPart, GraphStep, PlanGraph};
use crate::measure;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DraftPart {
    pub name: String,
    pub measure_kind: String,
    pub measure_unit: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DraftSupply {
    pub part_name: String,
    pub qty_value: String,
    pub qty_unit: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DraftFlow {
    pub part_name: String,
    pub qty_value: String,
    pub qty_unit: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DraftStep {
    pub name: String,
    pub planned_batches: i32,
    pub inputs: Vec<DraftFlow>,
    pub outputs: Vec<DraftFlow>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanDraft {
    pub parts: Vec<DraftPart>,
    pub presupplied: Vec<DraftSupply>,
    pub steps: Vec<DraftStep>,
}

pub struct EncodedPlan {
    pub parts_json: String,
    pub presupplied_json: String,
    pub steps_json: String,
}

#[derive(Deserialize)]
struct RawPart {
    #[serde(default)]
    name: String,
    #[serde(default)]
    measure_kind: String,
    #[serde(default)]
    measure_unit: String,
}

#[derive(Deserialize)]
struct RawSupply {
    #[serde(default)]
    part_name: String,
    #[serde(default)]
    qty_value: String,
    #[serde(default)]
    qty_unit: String,
}

#[derive(Deserialize)]
struct RawFlow {
    #[serde(default)]
    part_name: String,
    #[serde(default)]
    qty_value: String,
    #[serde(default)]
    qty_unit: String,
}

#[derive(Deserialize)]
struct RawStep {
    #[serde(default)]
    name: String,
    #[serde(default)]
    planned_batches: String,
    #[serde(default)]
    inputs: Vec<RawFlow>,
    #[serde(default)]
    outputs: Vec<RawFlow>,
}

#[derive(serde::Serialize)]
struct EncodedPart<'a> {
    name: &'a str,
    measure_kind: &'a str,
    measure_unit: &'a str,
}

#[derive(serde::Serialize)]
struct EncodedSupply {
    part_name: String,
    qty_value: String,
    qty_unit: String,
}

#[derive(serde::Serialize)]
struct EncodedFlow {
    part_name: String,
    qty_value: String,
    qty_unit: String,
}

#[derive(serde::Serialize)]
struct EncodedStep {
    name: String,
    planned_batches: String,
    inputs: Vec<EncodedFlow>,
    outputs: Vec<EncodedFlow>,
}

pub fn parse_plan(
    parts_json: &str,
    presupplied_json: &str,
    steps_json: &str,
) -> Result<PlanDraft, String> {
    let parts = parse_parts(parts_json)?;
    let presupplied = parse_supplies(&parts, presupplied_json)?;
    let steps = parse_steps(&parts, steps_json)?;
    let draft = PlanDraft {
        parts,
        presupplied,
        steps,
    };
    validate_graph(&draft)?;
    Ok(draft)
}

pub fn encode_plan(
    parts: &[part::Model],
    presupplied: &[presupplied_part::Model],
    steps: &[step::Model],
    inputs: &[step_input::Model],
    outputs: &[step_output::Model],
) -> EncodedPlan {
    let part_rows: Vec<EncodedPart<'_>> = parts
        .iter()
        .map(|row| EncodedPart {
            name: &row.name,
            measure_kind: &row.measure_kind,
            measure_unit: &row.measure_unit,
        })
        .collect();
    let supply_rows: Vec<EncodedSupply> = presupplied
        .iter()
        .map(|row| EncodedSupply {
            part_name: part_name(parts, row.part_id),
            qty_value: row.qty_value.normalize().to_string(),
            qty_unit: row.qty_unit.clone(),
        })
        .collect();
    let step_rows: Vec<EncodedStep> = steps
        .iter()
        .map(|row| EncodedStep {
            name: row.name.clone(),
            planned_batches: row.planned_batches.to_string(),
            inputs: inputs
                .iter()
                .filter(|line| line.step_id == row.id)
                .map(|line| encoded_flow(parts, line.part_id, line.qty_value, &line.qty_unit))
                .collect(),
            outputs: outputs
                .iter()
                .filter(|line| line.step_id == row.id)
                .map(|line| encoded_flow(parts, line.part_id, line.qty_value, &line.qty_unit))
                .collect(),
        })
        .collect();
    EncodedPlan {
        parts_json: serde_json::to_string(&part_rows).unwrap_or_else(|_| "[]".to_string()),
        presupplied_json: serde_json::to_string(&supply_rows).unwrap_or_else(|_| "[]".to_string()),
        steps_json: serde_json::to_string(&step_rows).unwrap_or_else(|_| "[]".to_string()),
    }
}

fn encoded_flow(parts: &[part::Model], part_id: i64, qty: Decimal, unit: &str) -> EncodedFlow {
    EncodedFlow {
        part_name: part_name(parts, part_id),
        qty_value: qty.normalize().to_string(),
        qty_unit: unit.to_string(),
    }
}

fn part_name(parts: &[part::Model], id: i64) -> String {
    parts
        .iter()
        .find(|row| row.id == id)
        .map(|row| row.name.clone())
        .unwrap_or_default()
}

fn parse_parts(raw: &str) -> Result<Vec<DraftPart>, String> {
    let rows: Vec<RawPart> = serde_json::from_str(&blank(raw))
        .map_err(|_| "Could not read the part lines.".to_string())?;
    let mut parts = Vec::with_capacity(rows.len());
    for row in rows {
        let name = require_name(&row.name)?;
        if parts
            .iter()
            .any(|part: &DraftPart| same_name(&part.name, &name))
        {
            return Err("A part with this name is already on the work order.".to_string());
        }
        let (kind, unit) = measure::part_measure(&row.measure_kind, &row.measure_unit)?;
        parts.push(DraftPart {
            name,
            measure_kind: kind.as_str().to_string(),
            measure_unit: unit,
        });
    }
    Ok(parts)
}

fn parse_supplies(parts: &[DraftPart], raw: &str) -> Result<Vec<DraftSupply>, String> {
    let rows: Vec<RawSupply> = serde_json::from_str(&blank(raw))
        .map_err(|_| "Could not read the presupplied parts.".to_string())?;
    let mut supplies = Vec::with_capacity(rows.len());
    for row in rows {
        let part = find_part(parts, &row.part_name)?;
        if supplies
            .iter()
            .any(|line: &DraftSupply| same_name(&line.part_name, &part.name))
        {
            return Err("That part is already supplied.".to_string());
        }
        let kind = measure::parse_kind(&part.measure_kind)?;
        let (qty, unit) =
            measure::parse_line(kind, &part.measure_unit, &row.qty_value, &row.qty_unit)?;
        supplies.push(DraftSupply {
            part_name: part.name.clone(),
            qty_value: qty.normalize().to_string(),
            qty_unit: unit,
        });
    }
    Ok(supplies)
}

fn parse_steps(parts: &[DraftPart], raw: &str) -> Result<Vec<DraftStep>, String> {
    let rows: Vec<RawStep> =
        serde_json::from_str(&blank(raw)).map_err(|_| "Could not read the steps.".to_string())?;
    let mut steps = Vec::with_capacity(rows.len());
    for row in rows {
        let name = require_name(&row.name)?;
        if steps
            .iter()
            .any(|step: &DraftStep| same_name(&step.name, &name))
        {
            return Err("A step with this name is already on the work order.".to_string());
        }
        let planned_batches = parse_batches(&row.planned_batches)?;
        let inputs = parse_flows(parts, &row.inputs)?;
        let outputs = parse_flows(parts, &row.outputs)?;
        if inputs.iter().any(|input| {
            outputs
                .iter()
                .any(|output| same_name(&input.part_name, &output.part_name))
        }) {
            return Err("A step cannot consume and produce the same part.".to_string());
        }
        steps.push(DraftStep {
            name,
            planned_batches,
            inputs,
            outputs,
        });
    }
    Ok(steps)
}

fn parse_flows(parts: &[DraftPart], rows: &[RawFlow]) -> Result<Vec<DraftFlow>, String> {
    let mut flows = Vec::with_capacity(rows.len());
    for row in rows {
        let part = find_part(parts, &row.part_name)?;
        if flows
            .iter()
            .any(|line: &DraftFlow| same_name(&line.part_name, &part.name))
        {
            return Err("That part is already on this side of the step.".to_string());
        }
        let kind = measure::parse_kind(&part.measure_kind)?;
        let (qty, unit) =
            measure::parse_line(kind, &part.measure_unit, &row.qty_value, &row.qty_unit)?;
        flows.push(DraftFlow {
            part_name: part.name.clone(),
            qty_value: qty.normalize().to_string(),
            qty_unit: unit,
        });
    }
    Ok(flows)
}

fn validate_graph(draft: &PlanDraft) -> Result<(), String> {
    let mut next_id = 1_i64;
    let mut graph_parts = Vec::with_capacity(draft.parts.len());
    for part in &draft.parts {
        graph_parts.push(GraphPart {
            id: next_id,
            name: part.name.clone(),
        });
        next_id = next_id.saturating_add(1);
    }
    let mut graph_steps = Vec::with_capacity(draft.steps.len());
    let mut inputs = Vec::new();
    let mut outputs = Vec::new();
    for step in &draft.steps {
        let step_id = next_id;
        next_id = next_id.saturating_add(1);
        graph_steps.push(GraphStep {
            id: step_id,
            name: step.name.clone(),
            planned_batches: step.planned_batches,
        });
        push_graph_flows(&graph_parts, draft, step_id, &step.inputs, &mut inputs)?;
        push_graph_flows(&graph_parts, draft, step_id, &step.outputs, &mut outputs)?;
    }
    let graph = PlanGraph {
        parts: graph_parts,
        steps: graph_steps,
        inputs,
        outputs,
        presupplied: Vec::new(),
    };
    graph::layout(&graph).map_err(|_| "This connection loops.".to_string())?;
    Ok(())
}

fn push_graph_flows(
    graph_parts: &[GraphPart],
    draft: &PlanDraft,
    step_id: i64,
    flows: &[DraftFlow],
    into: &mut Vec<GraphFlow>,
) -> Result<(), String> {
    for flow in flows {
        let part = find_part(&draft.parts, &flow.part_name)?;
        let kind = measure::parse_kind(&part.measure_kind)?;
        let qty = flow
            .qty_value
            .parse::<Decimal>()
            .map_err(|_| "Enter a quantity.".to_string())?;
        let qty = measure::in_part_unit(kind, &part.measure_unit, qty, &flow.qty_unit)?;
        let part_id = graph_parts
            .iter()
            .find(|row| same_name(&row.name, &part.name))
            .map(|row| row.id)
            .ok_or_else(|| "Choose a part from this work order.".to_string())?;
        into.push(GraphFlow {
            step_id,
            part_id,
            qty,
        });
    }
    Ok(())
}

fn find_part<'a>(parts: &'a [DraftPart], name: &str) -> Result<&'a DraftPart, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("Choose a part.".to_string());
    }
    parts
        .iter()
        .find(|part| same_name(&part.name, trimmed))
        .ok_or_else(|| "Choose a part from this work order.".to_string())
}

fn parse_batches(raw: &str) -> Result<i32, String> {
    let trimmed = raw.trim();
    let batches = trimmed
        .parse::<i32>()
        .map_err(|_| "Planned batches must be at least 1.".to_string())?;
    if batches <= 0 {
        return Err("Planned batches must be at least 1.".to_string());
    }
    Ok(batches)
}

fn blank(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        "[]".to_string()
    } else {
        trimmed.to_string()
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
