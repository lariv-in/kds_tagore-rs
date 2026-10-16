use super::handlers;
use super::keys::{MovementOutSelectModalKey, MovementOutSelectTableKey};

use lariv_plugin_finance_invoices::routes::FinanceInvoicesView;

lariv_core::define_plugin_routes! {
    plugin: InvoiceOutMovementTag;
    prefix: "/dashboard";
    routes: [
        get MovementOutFkSelectRouteTag, "/finance-invoices/out-movements/pick", handlers::select_out, fk_select(MovementOutSelectTableKey, MovementOutSelectModalKey), authorize(FinanceInvoicesView, []);
    ]
}
