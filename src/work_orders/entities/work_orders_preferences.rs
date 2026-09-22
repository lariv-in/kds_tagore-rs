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
    /// Typst + Minijinja template source for finalized work order PDFs (blank → default).
    pub work_order_pdf_template: Option<String>,
    /// Typst + Minijinja template source for quotation PDFs (blank → default).
    pub quotation_pdf_template: Option<String>,
    /// Quotation number format with finance-invoice placeholders (blank → default).
    pub quotation_number_format: Option<String>,
    /// Seller name on quotation PDFs.
    pub company_name: Option<String>,
    /// Seller address Typst markup on quotation PDFs.
    pub company_address: Option<String>,
    pub company_phone: Option<String>,
    pub company_gstin: Option<String>,
    pub place_of_supply: Option<String>,
    pub company_logo_vnode_id: Option<i64>,
    pub company_signature_vnode_id: Option<i64>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

pub type WorkOrdersPreferences = Model;
