use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "kds_work_order_completed_step_inputs")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub completed_step_id: i64,
    pub part_id: i64,
    #[sea_orm(column_type = "Decimal(Some((19, 6)))")]
    pub qty_value: Decimal,
    pub qty_unit: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::completed_step::Entity",
        from = "Column::CompletedStepId",
        to = "super::completed_step::Column::Id",
        on_delete = "Cascade"
    )]
    CompletedStep,
    #[sea_orm(
        belongs_to = "super::part::Entity",
        from = "Column::PartId",
        to = "super::part::Column::Id",
        on_delete = "Restrict"
    )]
    Part,
}

impl Related<super::completed_step::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::CompletedStep.def()
    }
}

impl Related<super::part::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Part.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
