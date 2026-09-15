use chrono::{DateTime, NaiveDate, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "kds_proforma_invoices")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub date: NaiveDate,
    pub customer_id: i64,
    pub invoice_number: String,
    pub work_order_id: Option<i64>,
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
    #[sea_orm(
        belongs_to = "super::work_order::Entity",
        from = "Column::WorkOrderId",
        to = "super::work_order::Column::Id",
        on_delete = "SetNull"
    )]
    WorkOrder,
    #[sea_orm(has_many = "super::proforma_invoice_machine_line::Entity")]
    MachineLines,
    #[sea_orm(has_many = "super::proforma_invoice_material_line::Entity")]
    MaterialLines,
}

impl Related<lariv_rs::plugins::customer::entities::customer::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Customer.def()
    }
}

impl Related<super::work_order::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::WorkOrder.def()
    }
}

impl Related<super::proforma_invoice_machine_line::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::MachineLines.def()
    }
}

impl Related<super::proforma_invoice_material_line::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::MaterialLines.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
