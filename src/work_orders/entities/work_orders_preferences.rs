use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "work_orders_preferences")]
/// SeaORM model row for the KDS Quotations singleton preferences (`id = 1`).
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    /// Typst + Minijinja template source for draft work order PDFs (blank → default).
    pub draft_work_order_pdf_template: Option<String>,
    /// Typst + Minijinja template source for quotation PDFs (blank → default).
    pub quotation_pdf_template: Option<String>,
    /// Quotation number format with finance-invoice placeholders (blank → default).
    pub quotation_number_format: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

pub type WorkOrdersPreferences = Model;
