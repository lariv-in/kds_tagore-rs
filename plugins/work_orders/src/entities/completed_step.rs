use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "kds_work_order_completed_steps")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub step_id: i64,
    pub completed_at: DateTime<Utc>,
    pub completed_by: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::step::Entity",
        from = "Column::StepId",
        to = "super::step::Column::Id",
        on_delete = "Restrict"
    )]
    Step,
    #[sea_orm(
        belongs_to = "lariv_plugin_users::entities::user::Entity",
        from = "Column::CompletedBy",
        to = "lariv_plugin_users::entities::user::Column::Id",
        on_delete = "Restrict"
    )]
    CompletedBy,
    #[sea_orm(has_many = "super::completed_step_input::Entity")]
    Inputs,
    #[sea_orm(has_many = "super::completed_step_output::Entity")]
    Outputs,
}

impl Related<super::step::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Step.def()
    }
}

impl Related<lariv_plugin_users::entities::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::CompletedBy.def()
    }
}

impl Related<super::completed_step_input::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Inputs.def()
    }
}

impl Related<super::completed_step_output::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Outputs.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
