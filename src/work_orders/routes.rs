use super::{handlers, keys::*};

lariv_rs::define_plugin_routes! {
    plugin: super::WorkOrdersTag;
    routes: [
        // Quotations (app default)
        get WorkOrdersDefaultRouteTag, "/work-orders", handlers::invoices_list, fragment(InvoiceTableKey);

        // Draft Work Orders
        get DraftWorkOrdersDefaultRouteTag, "/work-orders/orders", handlers::work_orders_list, fragment(WorkOrderTableKey);
        get WorkOrderFkSelectRouteTag, "/work-orders/orders/pick", handlers::work_order_select, fk_select(WorkOrderSelectTableKey, WorkOrderSelectModalKey);
        get WorkOrderCreateGetRouteTag, "/work-orders/orders/create", handlers::work_order_create_get, modal;
        post WorkOrderCreatePostRouteTag, "/work-orders/orders/create", handlers::work_order_create_post;
        get WorkOrderDetailRouteTag, "/work-orders/orders/{id}", handlers::work_order_detail;
        get WorkOrderEditGetRouteTag, "/work-orders/orders/{id}/edit", handlers::work_order_edit_get, modal;
        post WorkOrderEditPostRouteTag, "/work-orders/orders/{id}/edit", handlers::work_order_edit_post;
        get WorkOrderDeleteGetRouteTag, "/work-orders/orders/{id}/delete", handlers::work_order_delete_get, modal;
        post WorkOrderDeletePostRouteTag, "/work-orders/orders/{id}/delete", bare handlers::work_order_delete_post, fragment(WorkOrderDeleteModalKey);
        post WorkOrderConvertPostRouteTag, "/work-orders/orders/{id}/convert", bare handlers::work_order_convert_post, redirect;

        // Issued Work Orders
        get IssuedWorkOrdersRouteTag, "/work-orders/issued", handlers::issued_work_orders_list, fragment(IssuedWorkOrderTableKey);
        get IssuedWorkOrderDetailRouteTag, "/work-orders/issued/{id}", handlers::issued_work_order_detail;
        get IssuedWorkOrderDeleteGetRouteTag, "/work-orders/issued/{id}/delete", handlers::issued_work_order_delete_get, modal;
        post IssuedWorkOrderDeletePostRouteTag, "/work-orders/issued/{id}/delete", bare handlers::issued_work_order_delete_post, fragment(IssuedWorkOrderDeleteModalKey);
        post IssuedWorkOrderNewDraftPostRouteTag, "/work-orders/issued/{id}/new-draft", bare handlers::issued_work_order_new_draft_post, redirect;

        // Work Order Lines
        get WorkOrderLineEditGetRouteTag, "/work-orders/lines/{id}/edit", handlers::work_order_line_edit_get, modal;
        post WorkOrderLineEditPostRouteTag, "/work-orders/lines/{id}/edit", handlers::work_order_line_edit_post;
        get WorkOrderLineDeleteGetRouteTag, "/work-orders/lines/{id}/delete", handlers::work_order_line_delete_get, modal;
        post WorkOrderLineDeletePostRouteTag, "/work-orders/lines/{id}/delete", bare handlers::work_order_line_delete_post, fragment(WorkOrderLineDeleteModalKey);

        // Work Order Machine Lines
        get WorkOrderMachineLineEditGetRouteTag, "/work-orders/machine-lines/{id}/edit", handlers::work_order_machine_line_edit_get, modal;
        post WorkOrderMachineLineEditPostRouteTag, "/work-orders/machine-lines/{id}/edit", handlers::work_order_machine_line_edit_post;
        get WorkOrderMachineLineDeleteGetRouteTag, "/work-orders/machine-lines/{id}/delete", handlers::work_order_machine_line_delete_get, modal;
        post WorkOrderMachineLineDeletePostRouteTag, "/work-orders/machine-lines/{id}/delete", bare handlers::work_order_machine_line_delete_post, fragment(WorkOrderMachineLineDeleteModalKey);

        // Components
        get WorkOrdersComponentsRouteTag, "/work-orders/components", handlers::components_list, fragment(ComponentTableKey);
        get ComponentFkSelectRouteTag, "/work-orders/components/pick", handlers::component_select, fk_select(ComponentSelectTableKey, ComponentSelectModalKey);
        get ComponentCreateGetRouteTag, "/work-orders/components/create", handlers::component_create_get, modal;
        post ComponentCreatePostRouteTag, "/work-orders/components/create", handlers::component_create_post;
        get ComponentDetailRouteTag, "/work-orders/components/{id}", handlers::component_detail;
        get ComponentEditGetRouteTag, "/work-orders/components/{id}/edit", handlers::component_edit_get, modal;
        post ComponentEditPostRouteTag, "/work-orders/components/{id}/edit", handlers::component_edit_post;
        get ComponentDeleteGetRouteTag, "/work-orders/components/{id}/delete", handlers::component_delete_get, modal;
        post ComponentDeletePostRouteTag, "/work-orders/components/{id}/delete", bare handlers::component_delete_post, fragment(ComponentDeleteModalKey);

        // Quotations
        get WorkOrdersInvoicesRouteTag, "/work-orders/quotations", handlers::invoices_list, fragment(InvoiceTableKey);
        get InvoiceCreateGetRouteTag, "/work-orders/quotations/create", handlers::invoice_create_get, modal;
        post InvoiceCreatePostRouteTag, "/work-orders/quotations/create", handlers::invoice_create_post;
        get InvoiceDetailRouteTag, "/work-orders/quotations/{id}", handlers::invoice_detail;
        post InvoiceCreateWorkOrderPostRouteTag, "/work-orders/quotations/{id}/create-work-order", bare handlers::invoice_create_work_order_post, redirect;
        get InvoiceEditGetRouteTag, "/work-orders/quotations/{id}/edit", handlers::invoice_edit_get, modal;
        post InvoiceEditPostRouteTag, "/work-orders/quotations/{id}/edit", handlers::invoice_edit_post;
        get InvoiceDeleteGetRouteTag, "/work-orders/quotations/{id}/delete", handlers::invoice_delete_get, modal;
        post InvoiceDeletePostRouteTag, "/work-orders/quotations/{id}/delete", bare handlers::invoice_delete_post, fragment(InvoiceDeleteModalKey);

        // Preferences
        get WorkOrdersPrefsGetRouteTag, "/work-orders/preferences", handlers::preferences_get;
        post WorkOrdersPrefsPostRouteTag, "/work-orders/preferences", handlers::preferences_post;

        // PDFs
        get WorkOrderPdfModalRouteTag, "/work-orders/orders/{id}/pdf", bare handlers::work_order_pdf_modal, modal;
        get WorkOrderPdfRouteTag, "/work-orders/orders/{id}/pdf/file", bare handlers::work_order_pdf, file;
        get InvoicePdfModalRouteTag, "/work-orders/quotations/{id}/pdf", bare handlers::invoice_pdf_modal, modal;
        get InvoicePdfRouteTag, "/work-orders/quotations/{id}/pdf/file", bare handlers::invoice_pdf, file;
        post WorkOrderPdfPreviewPostRouteTag, "/work-orders/pdf/preview/work-order", bare handlers::work_order_pdf_preview_post, modal;
        post InvoicePdfPreviewPostRouteTag, "/work-orders/pdf/preview/quotation", bare handlers::invoice_pdf_preview_post, modal;
        get WorkOrdersPdfPreviewPdfRouteTag, "/work-orders/pdf/preview/{token}", bare handlers::preview_pdf_get, file, param token: String;

        // API
        post WorkOrdersCalculateApiRouteTag, "/work-orders/api/calculate", bare handlers::calculate_api, raw;
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
