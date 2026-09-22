use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "draft_work_order_machine_lines")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub draft_work_order_id: i64,
    pub machine_id: i64,
    pub variables: Json,
    #[sea_orm(column_type = "Decimal(Some((19, 6)))")]
    pub final_cost: Decimal,
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
        belongs_to = "crate::machinery_schedule::entities::machine::Entity",
        from = "Column::MachineId",
        to = "crate::machinery_schedule::entities::machine::Column::Id",
        on_delete = "Restrict"
    )]
    Machine,
}

impl Related<super::draft_work_order::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::DraftWorkOrder.def()
    }
}

impl Related<crate::machinery_schedule::entities::machine::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Machine.def()
    }
}

impl Model {
    pub fn line_total(&self) -> Decimal {
        self.final_cost
    }

    pub fn taxed_total(
        &self,
        taxes: &[lariv_rs::plugins::finance_taxes::entities::tax::Model],
    ) -> Decimal {
        crate::work_orders::tax_assoc::taxed_amount(self.line_total(), taxes)
    }

    pub fn format_variables_display(&self) -> String {
        crate::work_orders::line_vars::format_variables_display(
            &serde_json::json!({}),
            &self.variables,
            &serde_json::json!({}),
        )
    }
}

impl ActiveModelBehavior for ActiveModel {}
