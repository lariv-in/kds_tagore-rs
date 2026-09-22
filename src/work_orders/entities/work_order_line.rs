use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "kds_work_order_material_lines")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub work_order_id: i64,
    pub component_id: i64,
    pub variables: Json,
    #[sea_orm(column_type = "Decimal(Some((19, 6)))")]
    pub final_cost: Decimal,
    pub extra_data: Json,
}

impl Model {
    pub fn line_total(&self) -> Decimal {
        self.final_cost
    }

    pub fn taxed_total(
        &self,
        taxes: &[lariv_rs::plugins::finance_taxes::entities::tax::Model],
    ) -> Decimal {
        crate::work_orders::tax_assoc::taxed_amount(self.final_cost, taxes)
    }

    pub fn extra_data_str(&self) -> String {
        crate::work_orders::line_vars::extra_data_str(&self.extra_data)
    }

    pub fn dim_units_map(&self) -> std::collections::HashMap<String, String> {
        crate::work_orders::line_vars::dim_units_map(&self.extra_data)
    }

    pub fn dim_unit(&self) -> String {
        crate::work_orders::line_vars::dim_unit(&self.extra_data)
    }

    pub fn format_variables_display(&self) -> String {
        crate::work_orders::line_vars::format_variables_display(
            &serde_json::json!({}),
            &self.variables,
            &self.extra_data,
        )
    }

    pub fn format_variables_display_with_schema(&self, component_schema: &Json) -> String {
        crate::work_orders::line_vars::format_variables_display(
            component_schema,
            &self.variables,
            &self.extra_data,
        )
    }
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
    #[sea_orm(
        belongs_to = "super::component::Entity",
        from = "Column::ComponentId",
        to = "super::component::Column::Id",
        on_delete = "Restrict"
    )]
    Component,
}

impl Related<super::work_order::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::WorkOrder.def()
    }
}

impl Related<super::component::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Component.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
