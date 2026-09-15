use std::collections::HashMap;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "draft_work_order_material_lines")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub draft_work_order_id: i64,
    pub component_id: i64,
    pub variables: Json,
    #[sea_orm(column_type = "Decimal(Some((19, 6)))")]
    pub quantity: Decimal,
    #[sea_orm(column_type = "Decimal(Some((19, 6)))")]
    pub unit_weight: Decimal,
    #[sea_orm(column_type = "Decimal(Some((19, 6)))")]
    pub material_rate: Decimal,
    #[sea_orm(column_type = "Decimal(Some((19, 6)))")]
    pub final_cost: Decimal,
    pub extra_data: Json,
}

impl Model {
    /// Calculate final cost from quantity, material rate, and component unit weight:
    /// final_cost = quantity * material.rate * component.weight
    pub fn calculate_final_cost(quantity: Decimal, material_rate: Decimal, unit_weight: Decimal) -> Decimal {
        (quantity * material_rate * unit_weight).round_dp(2)
    }

    /// Return line total (equals final_cost).
    pub fn line_total(&self) -> Decimal {
        self.final_cost
    }

    /// Return variables as a map of dimension name to dimension value in mm.
    pub fn variables_map(&self) -> HashMap<String, f64> {
        serde_json::from_value(self.variables.clone()).unwrap_or_default()
    }

    /// Return extra_data as a JSON string.
    pub fn extra_data_str(&self) -> String {
        serde_json::to_string(&self.extra_data).unwrap_or_else(|_| "{}".into())
    }

    /// Return the map of variable dimension units from extra_data.
    pub fn dim_units_map(&self) -> HashMap<String, String> {
        if let serde_json::Value::Object(ref m) = self.extra_data {
            if let Some(serde_json::Value::Object(units)) = m.get("dim_units") {
                let mut map = HashMap::new();
                for (k, v) in units {
                    if let Some(s) = v.as_str() {
                        map.insert(k.clone(), s.to_string());
                    }
                }
                return map;
            } else if let Some(serde_json::Value::String(u)) = m.get("dim_unit") {
                let mut map = HashMap::new();
                map.insert("default".into(), u.clone());
                return map;
            }
        }
        HashMap::new()
    }

    /// Return the length unit used for dimensions (default "mm").
    pub fn dim_unit(&self) -> String {
        if let serde_json::Value::Object(ref m) = self.extra_data {
            if let Some(serde_json::Value::String(u)) = m.get("dim_unit") {
                return u.clone();
            }
        }
        "mm".into()
    }

    /// Format variables with their dimension names and per-variable units for user display.
    pub fn format_variables_display(&self) -> String {
        let vars = self.variables_map();
        if vars.is_empty() {
            return "-".into();
        }
        let units = self.dim_units_map();
        let default_unit = self.dim_unit();

        let mut keys: Vec<&String> = vars.keys().collect();
        keys.sort();
        let mut parts = Vec::new();
        for k in keys {
            let v_mm = vars[k];
            let unit = units.get(k).unwrap_or(&default_unit);
            let factor: f64 = match unit.as_str() {
                "cm" => 10.0,
                "m" => 1000.0,
                "km" => 1000000.0,
                "in" => 25.4,
                "ft" => 304.8,
                _ => 1.0,
            };
            if unit == "mm" {
                parts.push(format!("{k}: {v_mm} mm"));
            } else {
                let user_val = v_mm / factor;
                let user_str = if user_val.fract().abs() < 1e-4 {
                    format!("{:.0}", user_val)
                } else {
                    format!("{:.4}", user_val).trim_end_matches('0').trim_end_matches('.').to_string()
                };
                parts.push(format!("{k}: {user_str} {unit} ({v_mm} mm)"));
            }
        }
        parts.join(", ")
    }
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
        belongs_to = "super::component::Entity",
        from = "Column::ComponentId",
        to = "super::component::Column::Id",
        on_delete = "Restrict"
    )]
    Component,
}

impl Related<super::draft_work_order::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::DraftWorkOrder.def()
    }
}

impl Related<super::component::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Component.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
