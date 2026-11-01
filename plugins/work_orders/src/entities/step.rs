use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "kds_work_order_steps")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub work_order_id: i64,
    pub name: String,
    pub planned_batches: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::work_order::Entity",
        from = "Column::WorkOrderId",
        to = "super::work_order::Column::Id",
        on_delete = "Cascade"
    )]
    WorkOrder,
    #[sea_orm(has_many = "super::step_input::Entity")]
    Inputs,
    #[sea_orm(has_many = "super::step_output::Entity")]
    Outputs,
    #[sea_orm(has_many = "super::completed_step::Entity")]
    Completions,
}

impl Related<super::work_order::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::WorkOrder.def()
    }
}

impl Related<super::step_input::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Inputs.def()
    }
}

impl Related<super::step_output::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Outputs.def()
    }
}

impl Related<super::completed_step::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Completions.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
