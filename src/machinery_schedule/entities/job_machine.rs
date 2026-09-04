use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Join table (`Job` ↔ `Machine`).
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "machinery_job_machines")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub job_id: i64,
    #[sea_orm(primary_key, auto_increment = false)]
    pub machine_id: i64,
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
        belongs_to = "super::machine::Entity",
        from = "Column::MachineId",
        to = "super::machine::Column::Id",
        on_delete = "Restrict"
    )]
    Machine,
}

impl Related<super::job::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Job.def()
    }
}

impl Related<super::machine::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Machine.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
