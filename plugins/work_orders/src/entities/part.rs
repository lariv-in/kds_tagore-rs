use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "kds_work_order_parts")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub work_order_id: i64,
    pub name: String,
    pub measure_kind: String,
    pub measure_unit: String,
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
}

impl Related<super::work_order::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::WorkOrder.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
