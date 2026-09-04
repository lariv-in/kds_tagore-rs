use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use crate::machinery_schedule::duration::JobDuration;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "machinery_jobs")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub name: String,
    pub duration: JobDuration,
    pub progress: i16,
    pub order: i64,
    pub remarks: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_one = "super::completed_job::Entity")]
    CompletedJob,
    #[sea_orm(has_many = "super::job_machine::Entity")]
    JobMachines,
    #[sea_orm(has_many = "super::job_file::Entity")]
    JobFiles,
}

impl Related<super::completed_job::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::CompletedJob.def()
    }
}

impl Related<super::job_machine::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::JobMachines.def()
    }
}

impl Related<super::job_file::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::JobFiles.def()
    }
}

impl Related<super::machine::Entity> for Entity {
    fn to() -> RelationDef {
        super::job_machine::Relation::Machine.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::job_machine::Relation::Job.def().rev())
    }
}

impl Related<lariv_rs::plugins::filesystem::entities::filesystem_node::Entity> for Entity {
    fn to() -> RelationDef {
        super::job_file::Relation::VNode.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::job_file::Relation::Job.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}
