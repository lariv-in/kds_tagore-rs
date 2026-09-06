//! Import and export CRM leads in the KDS & Tagore Marketing Sheet XLSX format.

pub mod forms;
pub mod handlers;
pub mod import;
pub mod routes;
pub mod templates;
pub mod xlsx;

use lariv_rs::define_plugin_install;

/// Plugin identity tag.
pub struct MarketingSheetTag;

define_plugin_install! {
    plugin: MarketingSheetTag;
    steps: [
        templates(templates::Hook),
        slots(templates::SlotsHook),
        http(routes::Hook),
    ]
}
