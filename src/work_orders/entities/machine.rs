use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use rust_decimal::prelude::*;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "work_order_machines")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub name: String,
    /// Hourly machine rate stored as Decimal.
    #[sea_orm(column_type = "Decimal(Some((19, 6)))")]
    pub rate_decimal: Decimal,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::proforma_invoice_machine_line::Entity")]
    InvoiceMachineLines,
    #[sea_orm(has_many = "super::draft_work_order_machine_line::Entity")]
    DraftWorkOrderMachineLines,
}

impl Related<super::proforma_invoice_machine_line::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::InvoiceMachineLines.def()
    }
}

impl Related<super::draft_work_order_machine_line::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::DraftWorkOrderMachineLines.def()
    }
}

impl Model {
    /// Convert rate to (u64, u8) (Rupees and Paisa) as specified.
    pub fn rate(&self) -> (u64, u8) {
        decimal_to_rupees_paisa(self.rate_decimal)
    }

    /// Set rate from (u64, u8) (Rupees and Paisa).
    pub fn rate_from_tuple(rupees: u64, paisa: u8) -> Decimal {
        rupees_paisa_to_decimal(rupees, paisa)
    }
}

pub fn decimal_to_rupees_paisa(d: Decimal) -> (u64, u8) {
    let rupees = d.floor().to_u64().unwrap_or(0);
    let fract = d - Decimal::from(rupees);
    let paisa = (fract * Decimal::from(100)).round().to_u8().unwrap_or(0);
    (rupees, paisa.min(99))
}

pub fn rupees_paisa_to_decimal(rupees: u64, paisa: u8) -> Decimal {
    Decimal::from(rupees) + Decimal::from(paisa.min(99)) / Decimal::from(100)
}

impl ActiveModelBehavior for ActiveModel {}
