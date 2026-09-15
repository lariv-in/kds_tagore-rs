use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use rust_decimal::prelude::*;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "work_order_material_rates")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub material_id: i64,
    /// Rate in INR per kg stored with high decimal precision.
    #[sea_orm(column_type = "Decimal(Some((19, 6)))")]
    pub rate_decimal: Decimal,
    /// Datetime in UTC (representing IST when displayed).
    pub datetime: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::material::Entity",
        from = "Column::MaterialId",
        to = "super::material::Column::Id",
        on_delete = "Cascade"
    )]
    Material,
}

impl Related<super::material::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Material.def()
    }
}

impl Model {
    /// Return the rate as f64 (INR / kg) as requested in the specification.
    pub fn rate(&self) -> f64 {
        self.rate_decimal.to_f64().unwrap_or(0.0)
    }

    /// Format datetime in Indian Standard Time (UTC + 05:30).
    pub fn format_ist(&self) -> String {
        let ist = chrono::FixedOffset::east_opt(5 * 3600 + 30 * 60).unwrap();
        self.datetime.with_timezone(&ist).format("%Y-%m-%d %H:%M:%S IST").to_string()
    }
}

impl ActiveModelBehavior for ActiveModel {}
