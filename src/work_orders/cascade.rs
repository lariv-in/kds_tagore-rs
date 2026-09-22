//! Collect and delete a work order together with its linked machinery jobs.

use std::collections::BTreeSet;

use anyhow::{Result, anyhow};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter, TransactionTrait,
};

use crate::machinery_schedule::{
    entities::{
        completed_job::{self, Entity as CompletedJobEntity},
        job::{self, Entity as JobEntity},
    },
    routes::{CompletedJobDetailRouteTag, JobDetailRouteTag},
};
use crate::work_orders::{
    entities::work_order::{Entity as WorkOrderEntity, WORK_ORDER_SOURCE_DOC_TYPE},
    routes::IssuedWorkOrderDetailRouteTag,
    source_docs::work_order_label,
};

#[derive(Clone, Debug, Default)]
pub struct WorkOrderCascadeGraph {
    pub work_order_id: i64,
    pub job_ids: BTreeSet<i64>,
    pub completed_job_ids: BTreeSet<i64>,
}

#[derive(Clone, Debug)]
pub struct CascadeDeleteItem {
    pub kind: String,
    pub label: String,
    pub url: String,
}

/// Walk the work order and every machinery job that points at it.
pub async fn collect_work_order_cascade<C: ConnectionTrait>(
    db: &C,
    work_order_id: i64,
) -> Result<WorkOrderCascadeGraph> {
    let order = WorkOrderEntity::find_by_id(work_order_id)
        .one(db)
        .await?
        .ok_or_else(|| anyhow!("work order {work_order_id} not found"))?;

    let mut graph = WorkOrderCascadeGraph {
        work_order_id: order.id,
        job_ids: BTreeSet::new(),
        completed_job_ids: BTreeSet::new(),
    };
    if let Some(job_id) = order.job_id {
        graph.job_ids.insert(job_id);
    }

    let linked = JobEntity::find()
        .filter(job::Column::SourceDocType.eq(WORK_ORDER_SOURCE_DOC_TYPE))
        .filter(job::Column::SourceDocId.eq(order.id))
        .all(db)
        .await?;
    for job in linked {
        graph.job_ids.insert(job.id);
    }

    if !graph.job_ids.is_empty() {
        let completed = CompletedJobEntity::find()
            .filter(completed_job::Column::JobId.is_in(graph.job_ids.iter().copied()))
            .all(db)
            .await?;
        for row in completed {
            graph.completed_job_ids.insert(row.id);
        }
    }

    Ok(graph)
}

/// Labeled rows for the delete confirmation modal.
pub async fn cascade_delete_preview<C: ConnectionTrait>(
    db: &C,
    graph: &WorkOrderCascadeGraph,
) -> Result<Vec<CascadeDeleteItem>> {
    let mut items = Vec::new();

    if let Some(order) = WorkOrderEntity::find_by_id(graph.work_order_id)
        .one(db)
        .await?
    {
        items.push(CascadeDeleteItem {
            kind: "Work order".into(),
            label: work_order_label(&order),
            url: IssuedWorkOrderDetailRouteTag::new(order.id).url(),
        });
    }

    for id in &graph.job_ids {
        let Some(job) = JobEntity::find_by_id(*id).one(db).await? else {
            continue;
        };
        let label = if job.name.trim().is_empty() {
            format!("#{id}")
        } else {
            job.name
        };
        items.push(CascadeDeleteItem {
            kind: "Job".into(),
            label,
            url: JobDetailRouteTag::new(*id).url(),
        });
    }

    for id in &graph.completed_job_ids {
        let Some(completed) = CompletedJobEntity::find_by_id(*id).one(db).await? else {
            continue;
        };
        let job_name = JobEntity::find_by_id(completed.job_id)
            .one(db)
            .await?
            .map(|j| j.name)
            .filter(|n| !n.trim().is_empty())
            .unwrap_or_else(|| format!("#{}", completed.job_id));
        items.push(CascadeDeleteItem {
            kind: "Completed job".into(),
            label: job_name,
            url: CompletedJobDetailRouteTag::new(*id).url(),
        });
    }

    Ok(items)
}

/// Hard-delete a work order and every linked machinery job / completed job.
pub async fn delete_work_order_recursive(
    db: &DatabaseConnection,
    work_order_id: i64,
) -> Result<()> {
    let txn = db.begin().await?;
    let graph = collect_work_order_cascade(&txn, work_order_id).await?;
    WorkOrderEntity::delete_by_id(graph.work_order_id)
        .exec(&txn)
        .await?;
    for id in &graph.completed_job_ids {
        CompletedJobEntity::delete_by_id(*id).exec(&txn).await?;
    }
    for id in &graph.job_ids {
        JobEntity::delete_by_id(*id).exec(&txn).await?;
    }
    txn.commit().await?;
    Ok(())
}
