use super::handlers;
use super::keys::{MOVEMENT_OUT_SELECT_MODAL, MOVEMENT_OUT_SELECT_TABLE};

use lariv_plugin_finance_invoices::routes::FinanceInvoicesView;

lariv_core::define_plugin_routes! {
    plugin: InvoiceOutMovementTag;
    prefix: "/dashboard";
    routes: [
        get MovementOutFkSelectRouteTag, "/finance-invoices/out-movements/pick", handlers::select_out, fk_select(MOVEMENT_OUT_SELECT_TABLE, MOVEMENT_OUT_SELECT_MODAL), authorize(FinanceInvoicesView, []);
    ]
}
