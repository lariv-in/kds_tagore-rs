use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "delivery_preferences")]
/// Singleton Delivery preferences (`id = 1`).
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub company_name: Option<String>,
    /// Seller address as Typst markup.
    pub company_address: Option<String>,
    pub company_phone: Option<String>,
    pub company_email: Option<String>,
    pub company_gstin: Option<String>,
    /// Terms and conditions printed on delivery challan PDFs.
    pub terms_and_conditions: Option<String>,
    /// Challan number format with invoice-style placeholders (blank → default).
    pub challan_number_format: Option<String>,
    /// Typst + Minijinja template for delivery challan PDFs (blank → default).
    pub delivery_challan_pdf_template: Option<String>,
    pub logo_vnode_id: Option<i64>,
    pub signature_vnode_id: Option<i64>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

pub type DeliveryPreferences = Model;
