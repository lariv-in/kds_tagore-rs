//! GST Excel on the Accounting sidebar for this deployment.
//!
//! The GST export app tile stays superuser-only because its home page is preferences.

use lariv_plugin_finance_accounts::accounting_sidebar::{self, AccountingSidebarRegistrar};
use lariv_plugin_finance_gst_export::routes::GstExcelRouteTag;

#[derive(Clone, Copy, Default)]
pub struct Hook;

impl AccountingSidebarRegistrar for Hook {
    fn register_accounting_sidebar(
        self,
        cap: accounting_sidebar::AccountingSidebarRegistry,
    ) -> accounting_sidebar::AccountingSidebarRegistry {
        cap.push(accounting_sidebar::link::<GstExcelRouteTag>(
            "gst-excel",
            "GST Excel",
            130,
            Some("document-arrow-down"),
        ))
    }
}
