use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use rust_decimal::prelude::*;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use super::machine::{decimal_to_rupees_paisa, rupees_paisa_to_decimal};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "kds_proforma_invoice_material_lines")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub invoice_id: i64,
    pub material_id: Option<i64>,
    pub name: String,
    #[sea_orm(column_type = "Decimal(Some((19, 6)))")]
    pub rate_decimal: Decimal,
    #[sea_orm(column_type = "Decimal(Some((19, 6)))")]
    pub qty_decimal: Decimal,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::proforma_invoice::Entity",
        from = "Column::InvoiceId",
        to = "super::proforma_invoice::Column::Id",
        on_delete = "Cascade"
    )]
    Invoice,
    #[sea_orm(
        belongs_to = "super::material::Entity",
        from = "Column::MaterialId",
        to = "super::material::Column::Id",
        on_delete = "SetNull"
    )]
    Material,
}

impl Related<super::proforma_invoice::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Invoice.def()
    }
}

impl Related<super::material::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Material.def()
    }
}

impl Model {
    /// Return the rate as (u64, u8) (Rupees and Paisa per kg) as requested.
    pub fn rate(&self) -> (u64, u8) {
        decimal_to_rupees_paisa(self.rate_decimal)
    }

    /// Set rate from (u64, u8) (Rupees and Paisa per kg).
    pub fn rate_from_tuple(rupees: u64, paisa: u8) -> Decimal {
        rupees_paisa_to_decimal(rupees, paisa)
    }

    /// Return the quantity as (u64, u8) (Kg and grams) as requested.
    /// Note: u8 cannot exceed 255, so grams are clamped to 255.
    pub fn qty(&self) -> (u64, u8) {
        let kg = self.qty_decimal.floor().to_u64().unwrap_or(0);
        let fract = self.qty_decimal - Decimal::from(kg);
        let grams = (fract * Decimal::from(1000)).round().to_u64().unwrap_or(0);
        (kg, grams.min(255) as u8)
    }

    /// Return quantity as (u64, u16) (Kg and true 0..=999 grams without u8 overflow).
    pub fn qty_full_grams(&self) -> (u64, u16) {
        let kg = self.qty_decimal.floor().to_u64().unwrap_or(0);
        let fract = self.qty_decimal - Decimal::from(kg);
        let grams = (fract * Decimal::from(1000)).round().to_u16().unwrap_or(0);
        (kg, grams.min(999))
    }

    /// Set quantity from (u64, u8) (Kg and grams).
    pub fn qty_from_tuple(kg: u64, g: u8) -> Decimal {
        Decimal::from(kg) + Decimal::from(g) / Decimal::from(1000)
    }

    /// Calculate total amount for this line item in INR (rate * kg).
    pub fn line_total(&self) -> Decimal {
        (self.rate_decimal * self.qty_decimal).round_dp(2)
    }
}

impl ActiveModelBehavior for ActiveModel {}
