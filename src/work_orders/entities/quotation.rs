use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use crate::machinery_schedule::duration::JobDuration;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "kds_quotations")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub date: NaiveDate,
    pub customer_id: i64,
    pub invoice_number: String,
    pub duration: JobDuration,
}

impl Model {
    pub fn material_lines_total(&self, lines: &[super::quotation_material_line::Model]) -> Decimal {
        lines.iter().map(|l| l.line_total()).sum()
    }

    pub fn machine_lines_total(&self, ml: &[super::quotation_machine_line::Model]) -> Decimal {
        ml.iter().map(|l| l.line_total()).sum()
    }

    pub fn grand_total(
        &self,
        material_lines: &[super::quotation_material_line::Model],
        machine_lines: &[super::quotation_machine_line::Model],
    ) -> Decimal {
        self.material_lines_total(material_lines) + self.machine_lines_total(machine_lines)
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
    #[sea_orm(has_many = "super::quotation_machine_line::Entity")]
    MachineLines,
    #[sea_orm(has_many = "super::quotation_material_line::Entity")]
    MaterialLines,
    #[sea_orm(has_many = "super::draft_work_order::Entity")]
    DraftWorkOrders,
    #[sea_orm(has_many = "super::work_order::Entity")]
    WorkOrders,
}

impl Related<lariv_rs::plugins::customer::entities::customer::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Customer.def()
    }
}

impl Related<super::quotation_machine_line::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::MachineLines.def()
    }
}

impl Related<super::quotation_material_line::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::MaterialLines.def()
    }
}

impl Related<super::draft_work_order::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::DraftWorkOrders.def()
    }
}

impl Related<super::work_order::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::WorkOrders.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
