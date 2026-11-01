use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "kds_work_order_step_outputs")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub step_id: i64,
    pub part_id: i64,
    #[sea_orm(column_type = "Decimal(Some((19, 6)))")]
    pub qty_value: Decimal,
    pub qty_unit: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::step::Entity",
        from = "Column::StepId",
        to = "super::step::Column::Id",
        on_delete = "Cascade"
    )]
    Step,
    #[sea_orm(
        belongs_to = "super::part::Entity",
        from = "Column::PartId",
        to = "super::part::Column::Id",
        on_delete = "Restrict"
    )]
    Part,
}

impl Related<super::step::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Step.def()
    }
}

impl Related<super::part::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Part.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
