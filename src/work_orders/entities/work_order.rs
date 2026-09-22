use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use crate::machinery_schedule::duration::JobDuration;

/// Registry key stored on machinery jobs that originate from a work order.
pub const WORK_ORDER_SOURCE_DOC_TYPE: &str = "kds_wo.WorkOrder";

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "kds_work_orders")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub order_number: String,
    pub customer_id: i64,
    pub quotation_id: Option<i64>,
    pub job_id: Option<i64>,
    pub duration: JobDuration,
}

impl Model {
    pub fn total_amount(&self, lines: &[super::work_order_line::Model]) -> Decimal {
        lines.iter().map(|l| l.final_cost).sum()
    }

    pub fn total_amount_with_machine_lines(
        &self,
        lines: &[super::work_order_line::Model],
        machine_lines: &[super::work_order_machine_line::Model],
    ) -> Decimal {
        self.total_amount(lines)
            + machine_lines
                .iter()
                .map(|l| l.line_total())
                .sum::<Decimal>()
    }
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "lariv_rs::plugins::customer::entities::customer::Entity",
        from = "Column::CustomerId",
        to = "lariv_rs::plugins::customer::entities::customer::Column::Id",
        on_delete = "Restrict"
    )]
    Customer,
    #[sea_orm(has_many = "super::work_order_line::Entity")]
    MaterialLines,
    #[sea_orm(has_many = "super::work_order_machine_line::Entity")]
    MachineLines,
    #[sea_orm(
        belongs_to = "super::quotation::Entity",
        from = "Column::QuotationId",
        to = "super::quotation::Column::Id",
        on_delete = "SetNull"
    )]
    Quotation,
    #[sea_orm(
        belongs_to = "crate::machinery_schedule::entities::job::Entity",
        from = "Column::JobId",
        to = "crate::machinery_schedule::entities::job::Column::Id",
        on_delete = "SetNull"
    )]
    Job,
}

impl Related<lariv_rs::plugins::customer::entities::customer::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Customer.def()
    }
}

impl Related<super::work_order_line::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::MaterialLines.def()
    }
}

impl Related<super::work_order_machine_line::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::MachineLines.def()
    }
}

impl Related<super::quotation::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Quotation.def()
    }
}

impl Related<crate::machinery_schedule::entities::job::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Job.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
