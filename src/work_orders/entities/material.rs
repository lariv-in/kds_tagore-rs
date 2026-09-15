use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "work_order_materials")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub name: String,
    /// Material density in kg/m³.
    #[sea_orm(column_type = "Double")]
    pub density: f64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::material_rate::Entity")]
    Rates,
    #[sea_orm(has_many = "super::component::Entity")]
    Components,
    #[sea_orm(has_many = "super::proforma_invoice_material_line::Entity")]
    InvoiceMaterialLines,
}

impl Related<super::material_rate::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Rates.def()
    }
}

impl Related<super::component::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Components.def()
    }
}

impl Related<super::proforma_invoice_material_line::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::InvoiceMaterialLines.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
