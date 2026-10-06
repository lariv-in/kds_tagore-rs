//! Singleton Delivery preferences (`id = 1`).

use chrono::Utc;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, DatabaseConnection, EntityTrait};

use super::challan_number::DEFAULT_CHALLAN_NUMBER_FORMAT;
use super::entities::preferences::{self, DeliveryPreferences, Entity as PrefsEntity};
use super::pdf_templates::resolved_delivery_challan_pdf_template;

/// Load singleton preferences row (`id = 1`), creating it if missing.
pub async fn load_preferences(
    db: &DatabaseConnection,
) -> Result<DeliveryPreferences, sea_orm::DbErr> {
    if let Some(prefs) = PrefsEntity::find_by_id(1).one(db).await? {
        return Ok(prefs);
    }

    let now = Utc::now();
    let model = preferences::ActiveModel {
        id: Set(1),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        challan_number_format: Set(Some(DEFAULT_CHALLAN_NUMBER_FORMAT.to_string())),
        ..Default::default()
    };
    model.insert(db).await
}

/// Persist preferences fields onto the singleton row.
pub async fn save_preferences(
    db: &DatabaseConnection,
    prefs: DeliveryPreferences,
) -> Result<DeliveryPreferences, sea_orm::DbErr> {
    let mut am: preferences::ActiveModel = load_preferences(db).await?.into();
    am.company_name = Set(prefs.company_name);
    am.company_address = Set(prefs.company_address);
    am.company_phone = Set(prefs.company_phone);
    am.company_email = Set(prefs.company_email);
    am.company_gstin = Set(prefs.company_gstin);
    am.terms_and_conditions = Set(prefs.terms_and_conditions);
    am.challan_number_format = Set(prefs.challan_number_format);
    am.delivery_challan_pdf_template = Set(prefs.delivery_challan_pdf_template);
    am.logo_vnode_id = Set(prefs.logo_vnode_id);
    am.signature_vnode_id = Set(prefs.signature_vnode_id);
    am.updated_at = Set(Some(Utc::now()));
    am.update(db).await
}

/// Return the challan PDF template source, or the default if blank/None.
pub fn delivery_challan_pdf_template(prefs: &DeliveryPreferences) -> &str {
    resolved_delivery_challan_pdf_template(prefs.delivery_challan_pdf_template.as_deref())
}

/// Return the challan number format, or the default if blank/None.
pub fn challan_number_format(prefs: &DeliveryPreferences) -> &str {
    prefs
        .challan_number_format
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(DEFAULT_CHALLAN_NUMBER_FORMAT)
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

pub fn empty_preferences() -> DeliveryPreferences {
    DeliveryPreferences {
        id: 1,
        created_at: None,
        updated_at: None,
        company_name: None,
        company_address: None,
        company_phone: None,
        company_email: None,
        company_gstin: None,
        terms_and_conditions: None,
        challan_number_format: Some(DEFAULT_CHALLAN_NUMBER_FORMAT.to_string()),
        delivery_challan_pdf_template: None,
        logo_vnode_id: None,
        signature_vnode_id: None,
    }
}
