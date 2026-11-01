//! Part and step layout for one work order. Quantities are already in each part's unit.

use std::collections::{HashMap, HashSet};

use rust_decimal::Decimal;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PartRole {
    Raw,
    Intermediate,
    Demanded,
    Unused,
}

impl PartRole {
    pub fn label(self) -> &'static str {
        match self {
            Self::Raw => "Raw",
            Self::Intermediate => "Intermediate",
            Self::Demanded => "Demanded",
            Self::Unused => "Unused",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphPart {
    pub id: i64,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphStep {
    pub id: i64,
    pub name: String,
    pub planned_batches: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphFlow {
    pub step_id: i64,
    pub part_id: i64,
    pub qty: Decimal,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphSupply {
    pub part_id: i64,
    pub qty: Decimal,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanGraph {
    pub parts: Vec<GraphPart>,
    pub steps: Vec<GraphStep>,
    pub inputs: Vec<GraphFlow>,
    pub outputs: Vec<GraphFlow>,
    pub presupplied: Vec<GraphSupply>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RankedNode {
    pub id: i64,
    pub rank: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartCoverage {
    pub part_id: i64,
    pub supply: Decimal,
    pub demand: Decimal,
    pub gap: Decimal,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanLayout {
    pub roles: Vec<(i64, PartRole)>,
    pub part_ranks: Vec<RankedNode>,
    pub step_ranks: Vec<RankedNode>,
    pub coverage: Vec<PartCoverage>,
}

/// Ranks did not settle, so the plan contains a cycle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cycle;

/// Longest-path ranks. A cycle is a rank that keeps growing.
pub fn layout(plan: &PlanGraph) -> Result<PlanLayout, Cycle> {
    let mut part_rank: HashMap<i64, u32> = HashMap::new();
    for part in &plan.parts {
        part_rank.insert(part.id, 0);
    }
    let mut step_rank: HashMap<i64, u32> = HashMap::new();
    for step in &plan.steps {
        step_rank.insert(step.id, 0);
    }

    let limit = plan
        .parts
        .len()
        .saturating_add(plan.steps.len())
        .saturating_add(1);
    let mut guard = 0;
    loop {
        if guard > limit {
            return Err(Cycle);
        }
        guard = guard.saturating_add(1);
        let mut changed = false;
        for step in &plan.steps {
            let inputs = flows_for(&plan.inputs, step.id);
            let next = match max_part_rank(&part_rank, &inputs) {
                Some(rank) => rank.saturating_add(1),
                None => 0,
            };
            changed |= raise(&mut step_rank, step.id, next);
        }
        for flow in &plan.outputs {
            let Some(step_at) = step_rank.get(&flow.step_id).copied() else {
                continue;
            };
            changed |= raise(&mut part_rank, flow.part_id, step_at.saturating_add(1));
        }
        if !changed {
            break;
        }
    }

    let inputs: HashSet<i64> = plan.inputs.iter().map(|flow| flow.part_id).collect();
    let outputs: HashSet<i64> = plan.outputs.iter().map(|flow| flow.part_id).collect();
    let mut roles = Vec::with_capacity(plan.parts.len());
    for part in &plan.parts {
        let role = match (inputs.contains(&part.id), outputs.contains(&part.id)) {
            (true, false) => PartRole::Raw,
            (true, true) => PartRole::Intermediate,
            (false, true) => PartRole::Demanded,
            (false, false) => PartRole::Unused,
        };
        roles.push((part.id, role));
    }

    let mut part_ranks = Vec::with_capacity(part_rank.len());
    for (id, rank) in part_rank {
        part_ranks.push(RankedNode { id, rank });
    }
    part_ranks.sort_by_key(|node| (node.rank, node.id));
    let mut step_ranks = Vec::with_capacity(step_rank.len());
    for (id, rank) in step_rank {
        step_ranks.push(RankedNode { id, rank });
    }
    step_ranks.sort_by_key(|node| (node.rank, node.id));

    let coverage = plan
        .parts
        .iter()
        .map(|part| coverage_for(plan, part.id))
        .collect();
    Ok(PlanLayout {
        roles,
        part_ranks,
        step_ranks,
        coverage,
    })
}

pub fn role_of(layout: &PlanLayout, part_id: i64) -> Option<PartRole> {
    layout
        .roles
        .iter()
        .find(|(id, _)| *id == part_id)
        .map(|(_, role)| *role)
}

pub fn rank_of(nodes: &[RankedNode], id: i64) -> Option<u32> {
    nodes
        .iter()
        .find(|node| node.id == id)
        .map(|node| node.rank)
}

fn flows_for(flows: &[GraphFlow], step_id: i64) -> Vec<i64> {
    flows
        .iter()
        .filter(|flow| flow.step_id == step_id)
        .map(|flow| flow.part_id)
        .collect()
}

fn max_part_rank(ranks: &HashMap<i64, u32>, part_ids: &[i64]) -> Option<u32> {
    let mut max = None;
    for id in part_ids {
        if let Some(rank) = ranks.get(id) {
            max = Some(max.map_or(*rank, |current: u32| current.max(*rank)));
        }
    }
    max
}

fn raise(ranks: &mut HashMap<i64, u32>, id: i64, next: u32) -> bool {
    match ranks.get(&id).copied() {
        Some(current) if next > current => {
            ranks.insert(id, next);
            true
        }
        None => {
            ranks.insert(id, next);
            true
        }
        _ => false,
    }
}

fn coverage_for(plan: &PlanGraph, part_id: i64) -> PartCoverage {
    let mut supply = Decimal::ZERO;
    for row in &plan.presupplied {
        if row.part_id == part_id {
            supply += row.qty;
        }
    }
    for flow in &plan.outputs {
        if flow.part_id == part_id {
            supply += flow.qty * batches_of(plan, flow.step_id);
        }
    }
    let mut demand = Decimal::ZERO;
    for flow in &plan.inputs {
        if flow.part_id == part_id {
            demand += flow.qty * batches_of(plan, flow.step_id);
        }
    }
    PartCoverage {
        part_id,
        supply,
        demand,
        gap: supply - demand,
    }
}

fn batches_of(plan: &PlanGraph, step_id: i64) -> Decimal {
    let batches = plan
        .steps
        .iter()
        .find(|step| step.id == step_id)
        .map(|step| step.planned_batches)
        .unwrap_or(0);
    Decimal::from(batches.max(0))
}
