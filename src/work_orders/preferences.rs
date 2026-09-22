//! Singleton KDS Quotations preferences (`id = 1`) for numbering and PDF templates.

use chrono::Utc;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, DatabaseConnection, EntityTrait};

use super::entities::{
    WorkOrdersPreferences,
    work_orders_preferences::{self, Entity as PrefsEntity},
};
use super::pdf_templates::{
    resolved_draft_work_order_pdf_template, resolved_quotation_pdf_template,
    resolved_work_order_pdf_template,
};
use super::quotation_number::DEFAULT_QUOTATION_NUMBER_FORMAT;

/// Load singleton preferences row (`id = 1`), creating it if missing.
pub async fn load_preferences(
    db: &DatabaseConnection,
) -> Result<WorkOrdersPreferences, sea_orm::DbErr> {
    if let Some(prefs) = PrefsEntity::find_by_id(1).one(db).await? {
        return Ok(prefs);
    }

    let now = Utc::now();
    let model = work_orders_preferences::ActiveModel {
        id: Set(1),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        draft_work_order_pdf_template: Set(None),
        work_order_pdf_template: Set(None),
        quotation_pdf_template: Set(None),
        quotation_number_format: Set(Some(DEFAULT_QUOTATION_NUMBER_FORMAT.to_string())),
        ..Default::default()
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
    am.work_order_pdf_template = Set(prefs.work_order_pdf_template);
    am.quotation_pdf_template = Set(prefs.quotation_pdf_template);
    am.quotation_number_format = Set(prefs.quotation_number_format);
    am.company_name = Set(prefs.company_name);
    am.company_address = Set(prefs.company_address);
    am.company_phone = Set(prefs.company_phone);
    am.company_gstin = Set(prefs.company_gstin);
    am.place_of_supply = Set(prefs.place_of_supply);
    am.company_logo_vnode_id = Set(prefs.company_logo_vnode_id);
    am.company_signature_vnode_id = Set(prefs.company_signature_vnode_id);
    am.updated_at = Set(Some(Utc::now()));
    am.update(db).await
}

/// Return the draft work order PDF template source, or the default if blank/None.
///
/// Unmodified copies of the shipped example (including the pre-presentation
/// template) resolve to the current default so company preference fields render.
pub fn draft_work_order_pdf_template(prefs: &WorkOrdersPreferences) -> &str {
    resolved_draft_work_order_pdf_template(prefs.draft_work_order_pdf_template.as_deref())
}

/// Return the finalized work order PDF template source, or the default if blank/None.
///
/// Unmodified copies of the shipped example (including the pre-presentation
/// template) resolve to the current default so company preference fields render.
pub fn work_order_pdf_template(prefs: &WorkOrdersPreferences) -> &str {
    resolved_work_order_pdf_template(prefs.work_order_pdf_template.as_deref())
}

/// Return the quotation PDF template source, or the default if blank/None.
///
/// Unmodified copies of the shipped example (including the pre-presentation
/// template) resolve to the current default so company preference fields render.
pub fn quotation_pdf_template(prefs: &WorkOrdersPreferences) -> &str {
    resolved_quotation_pdf_template(prefs.quotation_pdf_template.as_deref())
}

/// Return the quotation number format, or the default if blank/None.
pub fn quotation_number_format(prefs: &WorkOrdersPreferences) -> &str {
    prefs
        .quotation_number_format
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(DEFAULT_QUOTATION_NUMBER_FORMAT)
}

/// Parse an optional text preference (blank → `None`).
pub fn opt_text(s: &str) -> Option<String> {
    let t = s.trim();
    if t.is_empty() {
        None
    } else {
        Some(t.to_string())
    }
}

/// Parse an optional filesystem VNode id (blank/0 → `None`).
pub fn opt_vnode_id(s: &str) -> Option<i64> {
    let t = s.trim();
    if t.is_empty() {
        return None;
    }
    t.parse().ok().filter(|&id| id > 0)
}

pub fn empty_preferences() -> WorkOrdersPreferences {
    WorkOrdersPreferences {
        id: 1,
        created_at: None,
        updated_at: None,
        draft_work_order_pdf_template: None,
        work_order_pdf_template: None,
        quotation_pdf_template: None,
        quotation_number_format: Some(DEFAULT_QUOTATION_NUMBER_FORMAT.to_string()),
        company_name: None,
        company_address: None,
        company_phone: None,
        company_gstin: None,
        place_of_supply: None,
        company_logo_vnode_id: None,
        company_signature_vnode_id: None,
    }
}
