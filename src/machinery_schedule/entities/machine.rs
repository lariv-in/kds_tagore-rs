use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "machinery_machines")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub name: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::job_machine::Entity")]
    JobMachines,
}

impl Related<super::job_machine::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::JobMachines.def()
    }
}

impl Related<super::job::Entity> for Entity {
    fn to() -> RelationDef {
        super::job_machine::Relation::Job.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::job_machine::Relation::Machine.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}
