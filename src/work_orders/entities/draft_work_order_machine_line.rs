use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use crate::machinery_schedule::duration::JobDuration;
use super::machine::{decimal_to_rupees_paisa, rupees_paisa_to_decimal};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "draft_work_order_machine_lines")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub draft_work_order_id: i64,
    pub machine_id: i64,
    /// Hourly machine rate for this line, pre-filled from the machine but manually editable.
    #[sea_orm(column_type = "Decimal(Some((19, 6)))")]
    pub rate_decimal: Decimal,
    /// Machine time used stored as nanoseconds.
    pub time_used: JobDuration,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::draft_work_order::Entity",
        from = "Column::DraftWorkOrderId",
        to = "super::draft_work_order::Column::Id",
        on_delete = "Cascade"
    )]
    DraftWorkOrder,
    #[sea_orm(
        belongs_to = "super::machine::Entity",
        from = "Column::MachineId",
        to = "super::machine::Column::Id",
        on_delete = "Restrict"
    )]
    Machine,
}

impl Related<super::draft_work_order::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::DraftWorkOrder.def()
    }
}

impl Related<super::machine::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Machine.def()
    }
}

impl Model {
    /// Return the rate as (u64, u8) (Rupees and Paisa per hour) as requested.
    pub fn rate(&self) -> (u64, u8) {
        decimal_to_rupees_paisa(self.rate_decimal)
    }

    /// Set rate from (u64, u8) (Rupees and Paisa per hour).
    pub fn rate_from_tuple(rupees: u64, paisa: u8) -> Decimal {
        rupees_paisa_to_decimal(rupees, paisa)
    }

    /// Calculate total amount for this line item in INR (rate * hours).
    pub fn line_total(&self) -> Decimal {
        let nanos = self.time_used.num_nanoseconds();
        let hours = Decimal::from(nanos) / Decimal::from(3_600_000_000_000i64);
        (self.rate_decimal * hours).round_dp(2)
    }
}

impl ActiveModelBehavior for ActiveModel {}