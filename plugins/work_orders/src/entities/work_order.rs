use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "kds_work_orders")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub title: String,
    pub customer_id: i64,
    pub folder_vnode_id: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "lariv_plugin_customer::entities::customer::Entity",
        from = "Column::CustomerId",
        to = "lariv_plugin_customer::entities::customer::Column::Id",
        on_delete = "Restrict"
    )]
    Customer,
    #[sea_orm(
        belongs_to = "lariv_plugin_filesystem::entities::filesystem_node::Entity",
        from = "Column::FolderVnodeId",
        to = "lariv_plugin_filesystem::entities::filesystem_node::Column::Id",
        on_delete = "Restrict"
    )]
    Folder,
    #[sea_orm(has_many = "super::part::Entity")]
    Parts,
    #[sea_orm(has_many = "super::step::Entity")]
    Steps,
    #[sea_orm(has_many = "super::presupplied_part::Entity")]
    PresuppliedParts,
}

impl Related<lariv_plugin_customer::entities::customer::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Customer.def()
    }
}

impl Related<lariv_plugin_filesystem::entities::filesystem_node::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Folder.def()
    }
}

impl Related<super::part::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Parts.def()
    }
}

impl Related<super::step::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Steps.def()
    }
}

impl Related<super::presupplied_part::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PresuppliedParts.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
