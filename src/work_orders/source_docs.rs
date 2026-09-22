//! Register work orders as machinery-schedule job source documents.

use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use sea_orm::{DatabaseConnection, EntityTrait};

use crate::machinery_schedule::{
    JobSourceDocInstance, JobSourceDocRegistrar, JobSourceDocRegistry, JobSourceDocType,
};
use crate::work_orders::{
    entities::work_order::{self, Entity as WorkOrderEntity, WORK_ORDER_SOURCE_DOC_TYPE},
    routes::IssuedWorkOrderDetailRouteTag,
};

#[derive(Clone, Copy, Default)]
pub struct Hook;

impl JobSourceDocRegistrar for Hook {
    fn register_job_source_docs(self, registry: JobSourceDocRegistry) -> JobSourceDocRegistry {
        registry.register(Arc::new(WorkOrderSourceDocType))
    }
}

fn work_order_instance_name(id: i64, order_number: &str) -> String {
    if order_number.trim().is_empty() {
        format!("#{id}")
    } else {
        order_number.to_string()
    }
}

struct WorkOrderSourceDocType;

struct WorkOrderInstance {
    id: i64,
    order_number: String,
}

impl JobSourceDocInstance for WorkOrderInstance {
    fn source_doc_type(&self) -> &str {
        WORK_ORDER_SOURCE_DOC_TYPE
    }

    fn source_doc_id(&self) -> i64 {
        self.id
    }

    fn display_name(&self) -> String {
        work_order_instance_name(self.id, &self.order_number)
    }

    fn detail_url(&self) -> String {
        IssuedWorkOrderDetailRouteTag::new(self.id).url()
    }
}

#[async_trait]
impl JobSourceDocType for WorkOrderSourceDocType {
    fn source_doc_type(&self) -> &str {
        WORK_ORDER_SOURCE_DOC_TYPE
    }

    fn display_name(&self) -> &str {
        "Work Order"
    }

    fn detail_url(&self, id: i64) -> String {
        IssuedWorkOrderDetailRouteTag::new(id).url()
    }

    async fn load_from_id(
        &self,
        db: &DatabaseConnection,
        id: i64,
    ) -> Result<Arc<dyn JobSourceDocInstance>> {
        let model = WorkOrderEntity::find_by_id(id)
            .one(db)
            .await?
            .with_context(|| format!("work order {id} not found"))?;
        Ok(Arc::new(WorkOrderInstance {
            id: model.id,
            order_number: model.order_number,
        }))
    }
}

pub fn work_order_label(order: &work_order::Model) -> String {
    work_order_instance_name(order.id, &order.order_number)
}
