use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "delivery_challan_lines")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub delivery_challan_id: i64,
    pub sr_no: i32,
    pub product_id: i64,
    /// `length`, `weight`, or `quantity` (a whole number).
    pub qty_kind: String,
    /// Length in millimetres when `qty_kind` is `length`.
    #[sea_orm(column_type = "Decimal(Some((15, 6)))")]
    pub qty_length: Option<Decimal>,
    /// Display unit for the length (`mm`, `cm`, `m`, `km`, `in`, `ft`).
    pub qty_length_unit: Option<String>,
    /// Weight in kilograms when `qty_kind` is `weight`.
    #[sea_orm(column_type = "Decimal(Some((19, 6)))")]
    pub qty_weight: Option<Decimal>,
    /// Whole-number quantity when `qty_kind` is `quantity`.
    pub qty_number: Option<i64>,
}

impl Model {
    pub fn qty_display(&self) -> String {
        crate::delivery::qty::format_stored_qty(
            &self.qty_kind,
            self.qty_length,
            self.qty_length_unit.as_deref(),
            self.qty_weight,
            self.qty_number,
        )
    }
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::delivery_challan::Entity",
        from = "Column::DeliveryChallanId",
        to = "super::delivery_challan::Column::Id",
        on_delete = "Cascade"
    )]
    DeliveryChallan,
    #[sea_orm(
        belongs_to = "lariv_rs::plugins::finance_products::entities::product::Entity",
        from = "Column::ProductId",
        to = "lariv_rs::plugins::finance_products::entities::product::Column::Id",
        on_delete = "Restrict"
    )]
    Product,
}

impl Related<super::delivery_challan::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::DeliveryChallan.def()
    }
}

impl Related<lariv_rs::plugins::finance_products::entities::product::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Product.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
