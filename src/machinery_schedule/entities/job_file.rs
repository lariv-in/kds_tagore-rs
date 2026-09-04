use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Join table (`Job` ↔ `VNode`).
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "machinery_job_files")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub job_id: i64,
    #[sea_orm(primary_key, auto_increment = false)]
    pub v_node_id: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::job::Entity",
        from = "Column::JobId",
        to = "super::job::Column::Id",
        on_delete = "Cascade"
    )]
    Job,
    #[sea_orm(
        belongs_to = "lariv_rs::plugins::filesystem::entities::filesystem_node::Entity",
        from = "Column::VNodeId",
        to = "lariv_rs::plugins::filesystem::entities::filesystem_node::Column::Id",
        on_delete = "Cascade"
    )]
    VNode,
}

impl Related<super::job::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Job.def()
    }
}

impl Related<lariv_rs::plugins::filesystem::entities::filesystem_node::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::VNode.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
