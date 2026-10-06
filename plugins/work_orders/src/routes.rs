use super::{handlers, keys::*, machines};

use kds_plugin_hr_role::Hr;
use lariv_plugin_hr::roles::Employee;

/// KDS Quotations routes. Allowlist is [`Hr`]; superuser always passes.
pub struct QuotationsAccess;

/// Quotation preferences page and sample PDFs. Allowlist is [`Employee`]; [`Hr`] is excluded. Superuser always passes.
pub struct QuotationsPreferencesView;

/// Quotation preferences save. Empty allowlist: superuser only.
pub struct QuotationsPreferencesMutate;

lariv_core::define_plugin_routes! {
    plugin: super::WorkOrdersTag;
    prefix: "/dashboard";
    routes: [
        // Quotations (app default)
        get WorkOrdersDefaultRouteTag, "/work-orders", handlers::invoices_list, fragment(InvoiceTableKey), authorize(QuotationsAccess, [Hr, Employee]);

        // Draft Work Orders
        get DraftWorkOrdersDefaultRouteTag, "/work-orders/orders", handlers::work_orders_list, fragment(WorkOrderTableKey), authorize(QuotationsAccess, [Hr, Employee]);
        get WorkOrderFkSelectRouteTag, "/work-orders/orders/pick", handlers::work_order_select, fk_select(WorkOrderSelectTableKey, WorkOrderSelectModalKey), authorize(QuotationsAccess, [Hr, Employee]);
        get WorkOrderCreateGetRouteTag, "/work-orders/orders/create", handlers::work_order_create_get, modal, authorize(QuotationsAccess, [Hr, Employee]);
        post WorkOrderCreatePostRouteTag, "/work-orders/orders/create", handlers::work_order_create_post, authorize(QuotationsAccess, [Hr, Employee]);
        get WorkOrderDetailRouteTag, "/work-orders/orders/{id}", handlers::work_order_detail, authorize(QuotationsAccess, [Hr, Employee]);
        get WorkOrderEditGetRouteTag, "/work-orders/orders/{id}/edit", handlers::work_order_edit_get, modal, authorize(QuotationsAccess, [Hr, Employee]);
        post WorkOrderEditPostRouteTag, "/work-orders/orders/{id}/edit", handlers::work_order_edit_post, authorize(QuotationsAccess, [Hr, Employee]);
        get WorkOrderDeleteGetRouteTag, "/work-orders/orders/{id}/delete", handlers::work_order_delete_get, modal, authorize(QuotationsAccess, [Hr, Employee]);
        post WorkOrderDeletePostRouteTag, "/work-orders/orders/{id}/delete", bare handlers::work_order_delete_post, fragment(WorkOrderDeleteModalKey), authorize(QuotationsAccess, [Hr, Employee]);
        post WorkOrderConvertPostRouteTag, "/work-orders/orders/{id}/convert", bare handlers::work_order_convert_post, redirect, authorize(QuotationsAccess, [Hr, Employee]);

        // Issued Work Orders
        get IssuedWorkOrdersRouteTag, "/work-orders/issued", handlers::issued_work_orders_list, fragment(IssuedWorkOrderTableKey), authorize(QuotationsAccess, [Hr, Employee]);
        get IssuedWorkOrderDetailRouteTag, "/work-orders/issued/{id}", handlers::issued_work_order_detail, authorize(QuotationsAccess, [Hr, Employee]);
        get IssuedWorkOrderDeleteGetRouteTag, "/work-orders/issued/{id}/delete", handlers::issued_work_order_delete_get, modal, authorize(QuotationsAccess, [Hr, Employee]);
        post IssuedWorkOrderDeletePostRouteTag, "/work-orders/issued/{id}/delete", bare handlers::issued_work_order_delete_post, fragment(IssuedWorkOrderDeleteModalKey), authorize(QuotationsAccess, [Hr, Employee]);
        post IssuedWorkOrderNewDraftPostRouteTag, "/work-orders/issued/{id}/new-draft", bare handlers::issued_work_order_new_draft_post, redirect, authorize(QuotationsAccess, [Hr, Employee]);
        get IssuedWorkOrderPdfModalRouteTag, "/work-orders/issued/{id}/pdf", bare handlers::issued_work_order_pdf_modal, modal, authorize(QuotationsAccess, [Hr, Employee]);
        get IssuedWorkOrderPdfRouteTag, "/work-orders/issued/{id}/pdf/file", bare handlers::issued_work_order_pdf, file, authorize(QuotationsAccess, [Hr, Employee]);

        // Work Order Lines
        get WorkOrderLineEditGetRouteTag, "/work-orders/lines/{id}/edit", handlers::work_order_line_edit_get, modal, authorize(QuotationsAccess, [Hr, Employee]);
        post WorkOrderLineEditPostRouteTag, "/work-orders/lines/{id}/edit", handlers::work_order_line_edit_post, authorize(QuotationsAccess, [Hr, Employee]);
        get WorkOrderLineDeleteGetRouteTag, "/work-orders/lines/{id}/delete", handlers::work_order_line_delete_get, modal, authorize(QuotationsAccess, [Hr, Employee]);
        post WorkOrderLineDeletePostRouteTag, "/work-orders/lines/{id}/delete", bare handlers::work_order_line_delete_post, fragment(WorkOrderLineDeleteModalKey), authorize(QuotationsAccess, [Hr, Employee]);

        // Work Order Machine Lines
        get WorkOrderMachineLineEditGetRouteTag, "/work-orders/machine-lines/{id}/edit", handlers::work_order_machine_line_edit_get, modal, authorize(QuotationsAccess, [Hr, Employee]);
        post WorkOrderMachineLineEditPostRouteTag, "/work-orders/machine-lines/{id}/edit", handlers::work_order_machine_line_edit_post, authorize(QuotationsAccess, [Hr, Employee]);
        get WorkOrderMachineLineDeleteGetRouteTag, "/work-orders/machine-lines/{id}/delete", handlers::work_order_machine_line_delete_get, modal, authorize(QuotationsAccess, [Hr, Employee]);
        post WorkOrderMachineLineDeletePostRouteTag, "/work-orders/machine-lines/{id}/delete", bare handlers::work_order_machine_line_delete_post, fragment(WorkOrderMachineLineDeleteModalKey), authorize(QuotationsAccess, [Hr, Employee]);

        // Components
        get WorkOrdersComponentsRouteTag, "/work-orders/components", handlers::components_list, fragment(ComponentTableKey), authorize(QuotationsAccess, [Hr, Employee]);
        get ComponentFkSelectRouteTag, "/work-orders/components/pick", handlers::component_select, fk_select(ComponentSelectTableKey, ComponentSelectModalKey), authorize(QuotationsAccess, [Hr, Employee]);
        get ComponentCreateGetRouteTag, "/work-orders/components/create", handlers::component_create_get, modal, authorize(QuotationsAccess, [Hr, Employee]);
        post ComponentCreatePostRouteTag, "/work-orders/components/create", handlers::component_create_post, authorize(QuotationsAccess, [Hr, Employee]);
        get ComponentDetailRouteTag, "/work-orders/components/{id}", handlers::component_detail, authorize(QuotationsAccess, [Hr, Employee]);
        get ComponentEditGetRouteTag, "/work-orders/components/{id}/edit", handlers::component_edit_get, modal, authorize(QuotationsAccess, [Hr, Employee]);
        post ComponentEditPostRouteTag, "/work-orders/components/{id}/edit", handlers::component_edit_post, authorize(QuotationsAccess, [Hr, Employee]);
        get ComponentDeleteGetRouteTag, "/work-orders/components/{id}/delete", handlers::component_delete_get, modal, authorize(QuotationsAccess, [Hr, Employee]);
        post ComponentDeletePostRouteTag, "/work-orders/components/{id}/delete", bare handlers::component_delete_post, fragment(ComponentDeleteModalKey), authorize(QuotationsAccess, [Hr, Employee]);

        // Machines (same records as Machinery Schedule, quotations sidebar)
        get WorkOrdersMachinesRouteTag, "/work-orders/machines", machines::list, fragment(WorkOrdersMachineTableKey), authorize(QuotationsAccess, [Hr, Employee]);
        get WorkOrdersMachineCreateGetRouteTag, "/work-orders/machines/create", machines::create_get, modal, authorize(QuotationsAccess, [Hr, Employee]);
        post WorkOrdersMachineCreatePostRouteTag, "/work-orders/machines/create", machines::create_post, authorize(QuotationsAccess, [Hr, Employee]);
        get WorkOrdersMachineDetailRouteTag, "/work-orders/machines/{id}", machines::detail, fragment(WorkOrdersMachineJobsTableKey), authorize(QuotationsAccess, [Hr, Employee]);
        get WorkOrdersMachineEditGetRouteTag, "/work-orders/machines/{id}/edit", machines::edit_get, modal, authorize(QuotationsAccess, [Hr, Employee]);
        post WorkOrdersMachineEditPostRouteTag, "/work-orders/machines/{id}/edit", machines::edit_post, authorize(QuotationsAccess, [Hr, Employee]);
        get WorkOrdersMachineDeleteGetRouteTag, "/work-orders/machines/{id}/delete", machines::delete_get, modal, authorize(QuotationsAccess, [Hr, Employee]);
        post WorkOrdersMachineDeletePostRouteTag, "/work-orders/machines/{id}/delete", bare machines::delete_post, fragment(WorkOrdersMachineDeleteModalKey), authorize(QuotationsAccess, [Hr, Employee]);

        // Quotations
        get WorkOrdersInvoicesRouteTag, "/work-orders/quotations", handlers::invoices_list, fragment(InvoiceTableKey), authorize(QuotationsAccess, [Hr, Employee]);
        get InvoiceCreateGetRouteTag, "/work-orders/quotations/create", handlers::invoice_create_get, modal, authorize(QuotationsAccess, [Hr, Employee]);
        post InvoiceCreatePostRouteTag, "/work-orders/quotations/create", handlers::invoice_create_post, authorize(QuotationsAccess, [Hr, Employee]);
        get InvoiceDetailRouteTag, "/work-orders/quotations/{id}", handlers::invoice_detail, authorize(QuotationsAccess, [Hr, Employee]);
        get InvoiceCreateWorkOrderGetRouteTag, "/work-orders/quotations/{id}/create-work-order", handlers::invoice_create_work_order_get, modal, authorize(QuotationsAccess, [Hr, Employee]);
        post InvoiceCreateWorkOrderPostRouteTag, "/work-orders/quotations/{id}/create-work-order", handlers::invoice_create_work_order_post, authorize(QuotationsAccess, [Hr, Employee]);
        get InvoiceEditGetRouteTag, "/work-orders/quotations/{id}/edit", handlers::invoice_edit_get, modal, authorize(QuotationsAccess, [Hr, Employee]);
        post InvoiceEditPostRouteTag, "/work-orders/quotations/{id}/edit", handlers::invoice_edit_post, authorize(QuotationsAccess, [Hr, Employee]);
        get InvoiceDeleteGetRouteTag, "/work-orders/quotations/{id}/delete", handlers::invoice_delete_get, modal, authorize(QuotationsAccess, [Hr, Employee]);
        post InvoiceDeletePostRouteTag, "/work-orders/quotations/{id}/delete", bare handlers::invoice_delete_post, fragment(InvoiceDeleteModalKey), authorize(QuotationsAccess, [Hr, Employee]);

        // Preferences
        get WorkOrdersPrefsGetRouteTag, "/work-orders/preferences", handlers::preferences_get, authorize(QuotationsPreferencesView, [Employee]);
        post WorkOrdersPrefsPostRouteTag, "/work-orders/preferences", handlers::preferences_post, authorize(QuotationsPreferencesMutate, []);

        // PDFs
        get WorkOrderPdfModalRouteTag, "/work-orders/orders/{id}/pdf", bare handlers::work_order_pdf_modal, modal, authorize(QuotationsAccess, [Hr, Employee]);
        get WorkOrderPdfRouteTag, "/work-orders/orders/{id}/pdf/file", bare handlers::work_order_pdf, file, authorize(QuotationsAccess, [Hr, Employee]);
        get InvoicePdfModalRouteTag, "/work-orders/quotations/{id}/pdf", bare handlers::invoice_pdf_modal, modal, authorize(QuotationsAccess, [Hr, Employee]);
        get InvoicePdfRouteTag, "/work-orders/quotations/{id}/pdf/file", bare handlers::invoice_pdf, file, authorize(QuotationsAccess, [Hr, Employee]);
        get InvoiceMailRouteTag, "/work-orders/quotations/{id}/mail", bare handlers::invoice_mail, file, authorize(QuotationsAccess, [Hr, Employee]);
        post WorkOrderPdfPreviewPostRouteTag, "/work-orders/pdf/preview/work-order", bare handlers::work_order_pdf_preview_post, modal, authorize(QuotationsPreferencesView, [Employee]);
        post IssuedWorkOrderPdfPreviewPostRouteTag, "/work-orders/pdf/preview/issued-work-order", bare handlers::issued_work_order_pdf_preview_post, modal, authorize(QuotationsPreferencesView, [Employee]);
        post InvoicePdfPreviewPostRouteTag, "/work-orders/pdf/preview/quotation", bare handlers::invoice_pdf_preview_post, modal, authorize(QuotationsPreferencesView, [Employee]);
        get WorkOrdersPdfPreviewPdfRouteTag, "/work-orders/pdf/preview/{token}", bare handlers::preview_pdf_get, file, param token: String, authorize(QuotationsPreferencesView, [Employee]);

        // API
        post WorkOrdersCalculateApiRouteTag, "/work-orders/api/calculate", bare handlers::calculate_api, raw, authorize(QuotationsAccess, [Hr, Employee]);
    ]
}

pub type DraftWorkOrderFkSelectRouteTag = WorkOrderFkSelectRouteTag;
pub type DraftWorkOrderCreateGetRouteTag = WorkOrderCreateGetRouteTag;
pub type DraftWorkOrderCreatePostRouteTag = WorkOrderCreatePostRouteTag;
pub type DraftWorkOrderDetailRouteTag = WorkOrderDetailRouteTag;
pub type DraftWorkOrderEditGetRouteTag = WorkOrderEditGetRouteTag;
pub type DraftWorkOrderEditPostRouteTag = WorkOrderEditPostRouteTag;
pub type DraftWorkOrderDeleteGetRouteTag = WorkOrderDeleteGetRouteTag;
pub type DraftWorkOrderDeletePostRouteTag = WorkOrderDeletePostRouteTag;

pub type DraftWorkOrderLineEditGetRouteTag = WorkOrderLineEditGetRouteTag;
pub type DraftWorkOrderLineEditPostRouteTag = WorkOrderLineEditPostRouteTag;
pub type DraftWorkOrderLineDeleteGetRouteTag = WorkOrderLineDeleteGetRouteTag;
pub type DraftWorkOrderLineDeletePostRouteTag = WorkOrderLineDeletePostRouteTag;

pub type DraftWorkOrderMaterialLineEditGetRouteTag = WorkOrderLineEditGetRouteTag;
pub type DraftWorkOrderMaterialLineEditPostRouteTag = WorkOrderLineEditPostRouteTag;
pub type DraftWorkOrderMaterialLineDeleteGetRouteTag = WorkOrderLineDeleteGetRouteTag;
pub type DraftWorkOrderMaterialLineDeletePostRouteTag = WorkOrderLineDeletePostRouteTag;

pub type WorkOrderMaterialLineEditGetRouteTag = WorkOrderLineEditGetRouteTag;
pub type WorkOrderMaterialLineEditPostRouteTag = WorkOrderLineEditPostRouteTag;
pub type WorkOrderMaterialLineDeleteGetRouteTag = WorkOrderLineDeleteGetRouteTag;
pub type WorkOrderMaterialLineDeletePostRouteTag = WorkOrderLineDeletePostRouteTag;

pub type DraftWorkOrderMachineLineEditGetRouteTag = WorkOrderMachineLineEditGetRouteTag;
pub type DraftWorkOrderMachineLineEditPostRouteTag = WorkOrderMachineLineEditPostRouteTag;
pub type DraftWorkOrderMachineLineDeleteGetRouteTag = WorkOrderMachineLineDeleteGetRouteTag;
pub type DraftWorkOrderMachineLineDeletePostRouteTag = WorkOrderMachineLineDeletePostRouteTag;
