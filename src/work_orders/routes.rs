use super::{handlers, keys::*};

lariv_rs::define_plugin_routes! {
    plugin: super::WorkOrdersTag;
    routes: [
        // Work Orders
        get WorkOrdersDefaultRouteTag, "/work-orders", handlers::work_orders_list, fragment(WorkOrderTableKey);
        get WorkOrderFkSelectRouteTag, "/work-orders/orders/pick", handlers::work_order_select, fk_select(WorkOrderSelectTableKey, WorkOrderSelectModalKey);
        get WorkOrderCreateGetRouteTag, "/work-orders/orders/create", handlers::work_order_create_get, modal;
        post WorkOrderCreatePostRouteTag, "/work-orders/orders/create", handlers::work_order_create_post;
        get WorkOrderDetailRouteTag, "/work-orders/orders/{id}", handlers::work_order_detail;
        get WorkOrderEditGetRouteTag, "/work-orders/orders/{id}/edit", handlers::work_order_edit_get, modal;
        post WorkOrderEditPostRouteTag, "/work-orders/orders/{id}/edit", handlers::work_order_edit_post;
        get WorkOrderDeleteGetRouteTag, "/work-orders/orders/{id}/delete", handlers::work_order_delete_get, modal;
        post WorkOrderDeletePostRouteTag, "/work-orders/orders/{id}/delete", bare handlers::work_order_delete_post, fragment(WorkOrderDeleteModalKey);

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

        // Shapes
        get WorkOrdersShapesRouteTag, "/work-orders/shapes", handlers::shapes_list, fragment(ShapeTableKey);
        get ShapeFkSelectRouteTag, "/work-orders/shapes/pick", handlers::shape_select, fk_select(ShapeSelectTableKey, ShapeSelectModalKey);
        get ShapeCreateGetRouteTag, "/work-orders/shapes/create", handlers::shape_create_get, modal;
        post ShapeCreatePostRouteTag, "/work-orders/shapes/create", handlers::shape_create_post;
        get ShapeDetailRouteTag, "/work-orders/shapes/{id}", handlers::shape_detail;
        get ShapeEditGetRouteTag, "/work-orders/shapes/{id}/edit", handlers::shape_edit_get, modal;
        post ShapeEditPostRouteTag, "/work-orders/shapes/{id}/edit", handlers::shape_edit_post;
        get ShapeDeleteGetRouteTag, "/work-orders/shapes/{id}/delete", handlers::shape_delete_get, modal;
        post ShapeDeletePostRouteTag, "/work-orders/shapes/{id}/delete", bare handlers::shape_delete_post, fragment(ShapeDeleteModalKey);

        // Materials
        get WorkOrdersMaterialsRouteTag, "/work-orders/materials", handlers::materials_list, fragment(MaterialTableKey);
        get MaterialFkSelectRouteTag, "/work-orders/materials/pick", handlers::material_select, fk_select(MaterialSelectTableKey, MaterialSelectModalKey);
        get MaterialCreateGetRouteTag, "/work-orders/materials/create", handlers::material_create_get, modal;
        post MaterialCreatePostRouteTag, "/work-orders/materials/create", handlers::material_create_post;
        get MaterialDetailRouteTag, "/work-orders/materials/{id}", handlers::material_detail;
        get MaterialEditGetRouteTag, "/work-orders/materials/{id}/edit", handlers::material_edit_get, modal;
        post MaterialEditPostRouteTag, "/work-orders/materials/{id}/edit", handlers::material_edit_post;
        get MaterialDeleteGetRouteTag, "/work-orders/materials/{id}/delete", handlers::material_delete_get, modal;
        post MaterialDeletePostRouteTag, "/work-orders/materials/{id}/delete", bare handlers::material_delete_post, fragment(MaterialDeleteModalKey);

        // Material Rates
        get WorkOrdersRatesRouteTag, "/work-orders/rates", handlers::rates_list, fragment(MaterialRateTableKey);
        get MaterialRateCreateGetRouteTag, "/work-orders/rates/create", handlers::rate_create_get, modal;
        post MaterialRateCreatePostRouteTag, "/work-orders/rates/create", handlers::rate_create_post;
        get MaterialRateDeleteGetRouteTag, "/work-orders/rates/{id}/delete", handlers::rate_delete_get, modal;
        post MaterialRateDeletePostRouteTag, "/work-orders/rates/{id}/delete", bare handlers::rate_delete_post, fragment(MaterialRateDeleteModalKey);

        // Machines
        get WorkOrdersMachinesRouteTag, "/work-orders/machines", handlers::machines_list, fragment(MachineTableKey);
        get MachineCreateGetRouteTag, "/work-orders/machines/create", handlers::machine_create_get, modal;
        post MachineCreatePostRouteTag, "/work-orders/machines/create", handlers::machine_create_post;
        get MachineDetailRouteTag, "/work-orders/machines/{id}", handlers::machine_detail;
        get MachineEditGetRouteTag, "/work-orders/machines/{id}/edit", handlers::machine_edit_get, modal;
        post MachineEditPostRouteTag, "/work-orders/machines/{id}/edit", handlers::machine_edit_post;
        get MachineDeleteGetRouteTag, "/work-orders/machines/{id}/delete", handlers::machine_delete_get, modal;
        post MachineDeletePostRouteTag, "/work-orders/machines/{id}/delete", bare handlers::machine_delete_post, fragment(MachineDeleteModalKey);
        get MachineFkSelectRouteTag, "/work-orders/machines/pick", handlers::machine_select, fk_select(MachineSelectTableKey, MachineSelectModalKey);

        // Proforma Invoices
        get WorkOrdersInvoicesRouteTag, "/work-orders/invoices", handlers::invoices_list, fragment(InvoiceTableKey);
        get InvoiceCreateGetRouteTag, "/work-orders/invoices/create", handlers::invoice_create_get, modal;
        post InvoiceCreatePostRouteTag, "/work-orders/invoices/create", handlers::invoice_create_post;
        get InvoiceDetailRouteTag, "/work-orders/invoices/{id}", handlers::invoice_detail;
        get InvoiceEditGetRouteTag, "/work-orders/invoices/{id}/edit", handlers::invoice_edit_get, modal;
        post InvoiceEditPostRouteTag, "/work-orders/invoices/{id}/edit", handlers::invoice_edit_post;
        get InvoiceDeleteGetRouteTag, "/work-orders/invoices/{id}/delete", handlers::invoice_delete_get, modal;
        post InvoiceDeletePostRouteTag, "/work-orders/invoices/{id}/delete", bare handlers::invoice_delete_post, fragment(InvoiceDeleteModalKey);

        // Preferences
        get WorkOrdersPrefsGetRouteTag, "/work-orders/preferences", handlers::preferences_get;
        post WorkOrdersPrefsPostRouteTag, "/work-orders/preferences", handlers::preferences_post;

        // PDFs
        get WorkOrderPdfRouteTag, "/work-orders/orders/{id}/pdf", bare handlers::work_order_pdf, file;
        get InvoicePdfRouteTag, "/work-orders/invoices/{id}/pdf", bare handlers::invoice_pdf, file;
        post WorkOrderPdfPreviewPostRouteTag, "/work-orders/pdf/preview/work-order", bare handlers::work_order_pdf_preview_post, modal;
        post InvoicePdfPreviewPostRouteTag, "/work-orders/pdf/preview/invoice", bare handlers::invoice_pdf_preview_post, modal;
        get WorkOrdersPdfPreviewPdfRouteTag, "/work-orders/pdf/preview/{token}", bare handlers::preview_pdf_get, file, param token: String;

        // API
        post WorkOrdersCalculateApiRouteTag, "/work-orders/api/calculate", bare handlers::calculate_api, raw;
    ]
}

pub type DraftWorkOrdersDefaultRouteTag = WorkOrdersDefaultRouteTag;
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

pub type DraftMachineFkSelectRouteTag = MachineFkSelectRouteTag;

