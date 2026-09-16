//! Singleton Work Orders preferences (`id = 1`) for PDF templates.

use chrono::Utc;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, DatabaseConnection, EntityTrait};

use super::entities::{
    WorkOrdersPreferences,
    work_orders_preferences::{self, Entity as PrefsEntity},
};
use super::pdf_templates::{DEFAULT_DRAFT_WORK_ORDER_PDF_TEMPLATE, DEFAULT_PROFORMA_INVOICE_PDF_TEMPLATE};

/// Load singleton preferences row (`id = 1`), creating it if missing.
pub async fn load_preferences(db: &DatabaseConnection) -> Result<WorkOrdersPreferences, sea_orm::DbErr> {
    if let Some(prefs) = PrefsEntity::find_by_id(1).one(db).await? {
        return Ok(prefs);
    }

    let now = Utc::now();
    let model = work_orders_preferences::ActiveModel {
        id: Set(1),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        draft_work_order_pdf_template: Set(Some(DEFAULT_DRAFT_WORK_ORDER_PDF_TEMPLATE.to_string())),
        proforma_invoice_pdf_template: Set(Some(DEFAULT_PROFORMA_INVOICE_PDF_TEMPLATE.to_string())),
    };
    model.insert(db).await
}

/// Persist preferences fields onto the singleton row.
pub async fn save_preferences(
    db: &DatabaseConnection,
    prefs: WorkOrdersPreferences,
) -> Result<WorkOrdersPreferences, sea_orm::DbErr> {
    let mut am: work_orders_preferences::ActiveModel = load_preferences(db).await?.into();
    am.draft_work_order_pdf_template = Set(prefs.draft_work_order_pdf_template);
    am.proforma_invoice_pdf_template = Set(prefs.proforma_invoice_pdf_template);
    am.updated_at = Set(Some(Utc::now()));
    am.update(db).await
}

/// Return the draft work order PDF template source, or the default if blank/None.
pub fn draft_work_order_pdf_template(prefs: &WorkOrdersPreferences) -> &str {
    prefs
        .draft_work_order_pdf_template
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(DEFAULT_DRAFT_WORK_ORDER_PDF_TEMPLATE)
}

/// Return the proforma invoice PDF template source, or the default if blank/None.
pub fn proforma_invoice_pdf_template(prefs: &WorkOrdersPreferences) -> &str {
    prefs
        .proforma_invoice_pdf_template
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(DEFAULT_PROFORMA_INVOICE_PDF_TEMPLATE)
}

pub fn empty_preferences() -> WorkOrdersPreferences {
    WorkOrdersPreferences {
        id: 1,
        created_at: None,
        updated_at: None,
        draft_work_order_pdf_template: Some(DEFAULT_DRAFT_WORK_ORDER_PDF_TEMPLATE.to_string()),
        proforma_invoice_pdf_template: Some(DEFAULT_PROFORMA_INVOICE_PDF_TEMPLATE.to_string()),
    }
}
