lariv_core::swap_key!(DraftWorkOrderTableKey, "wo-draft-order-table");
lariv_core::swap_key!(DraftWorkOrderCreateModalKey, "wo-draft-order-create-modal");
lariv_core::swap_key!(DraftWorkOrderEditModalKey, "wo-draft-order-edit-modal");
lariv_core::swap_key!(DraftWorkOrderDeleteModalKey, "wo-draft-order-delete-modal");
lariv_core::swap_key!(DraftWorkOrderSelectTableKey, "wo-draft-order-select-table");
lariv_core::swap_key!(DraftWorkOrderSelectModalKey, "wo-draft-order-select-modal");
lariv_core::swap_key!(
    DraftWorkOrderMaterialLinesTableKey,
    "wo-draft-order-material-lines-table"
);
lariv_core::swap_key!(
    DraftWorkOrderMaterialLineEditModalKey,
    "wo-draft-order-material-line-edit-modal"
);
lariv_core::swap_key!(
    DraftWorkOrderMaterialLineDeleteModalKey,
    "wo-draft-order-material-line-delete-modal"
);

lariv_core::swap_key!(
    DraftWorkOrderMachineLinesTableKey,
    "wo-draft-order-machine-lines-table"
);
lariv_core::swap_key!(
    DraftWorkOrderMachineLineEditModalKey,
    "wo-draft-order-machine-line-edit-modal"
);
lariv_core::swap_key!(
    DraftWorkOrderMachineLineDeleteModalKey,
    "wo-draft-order-machine-line-delete-modal"
);

pub type DraftWorkOrderLinesTableKey = DraftWorkOrderMaterialLinesTableKey;
pub type DraftWorkOrderLineEditModalKey = DraftWorkOrderMaterialLineEditModalKey;
pub type DraftWorkOrderLineDeleteModalKey = DraftWorkOrderMaterialLineDeleteModalKey;

pub type WorkOrderTableKey = DraftWorkOrderTableKey;
pub type WorkOrderCreateModalKey = DraftWorkOrderCreateModalKey;
pub type WorkOrderEditModalKey = DraftWorkOrderEditModalKey;
pub type WorkOrderDeleteModalKey = DraftWorkOrderDeleteModalKey;
pub type WorkOrderSelectTableKey = DraftWorkOrderSelectTableKey;
pub type WorkOrderSelectModalKey = DraftWorkOrderSelectModalKey;
pub type WorkOrderLinesTableKey = DraftWorkOrderMaterialLinesTableKey;
pub type WorkOrderLineEditModalKey = DraftWorkOrderMaterialLineEditModalKey;
pub type WorkOrderLineDeleteModalKey = DraftWorkOrderMaterialLineDeleteModalKey;

pub type WorkOrderMaterialLinesTableKey = DraftWorkOrderMaterialLinesTableKey;
pub type WorkOrderMaterialLineEditModalKey = DraftWorkOrderMaterialLineEditModalKey;
pub type WorkOrderMaterialLineDeleteModalKey = DraftWorkOrderMaterialLineDeleteModalKey;

pub type WorkOrderItemsTableKey = DraftWorkOrderMaterialLinesTableKey;
pub type WorkOrderItemEditModalKey = DraftWorkOrderMaterialLineEditModalKey;
pub type WorkOrderItemDeleteModalKey = DraftWorkOrderMaterialLineDeleteModalKey;

pub type WorkOrderMachineLinesTableKey = DraftWorkOrderMachineLinesTableKey;
pub type WorkOrderMachineLineEditModalKey = DraftWorkOrderMachineLineEditModalKey;
pub type WorkOrderMachineLineDeleteModalKey = DraftWorkOrderMachineLineDeleteModalKey;

lariv_core::swap_key!(ComponentTableKey, "wo-component-table");
lariv_core::swap_key!(ComponentCreateModalKey, "wo-component-create-modal");
lariv_core::swap_key!(ComponentEditModalKey, "wo-component-edit-modal");
lariv_core::swap_key!(ComponentDeleteModalKey, "wo-component-delete-modal");
lariv_core::swap_key!(ComponentSelectTableKey, "wo-component-select-table");
lariv_core::swap_key!(ComponentSelectModalKey, "wo-component-select-modal");

lariv_core::swap_key!(InvoiceTableKey, "wo-invoice-table");
lariv_core::swap_key!(InvoiceCreateModalKey, "wo-invoice-create-modal");
lariv_core::swap_key!(InvoiceEditModalKey, "wo-invoice-edit-modal");
lariv_core::swap_key!(InvoiceDeleteModalKey, "wo-invoice-delete-modal");
lariv_core::swap_key!(
    InvoiceCreateWorkOrderModalKey,
    "wo-invoice-create-work-order-modal"
);

lariv_core::swap_key!(IssuedWorkOrderTableKey, "wo-issued-order-table");
lariv_core::swap_key!(
    IssuedWorkOrderDeleteModalKey,
    "wo-issued-order-delete-modal"
);

lariv_core::swap_key!(WorkOrdersPdfPreviewModalKey, "wo-pdf-preview-modal");
lariv_core::swap_key!(WorkOrdersPdfModalKey, "wo-pdf-modal");

lariv_core::swap_key!(WorkOrdersMachineTableKey, "wo-machine-table");
lariv_core::swap_key!(WorkOrdersMachineJobsTableKey, "wo-machine-jobs-table");
lariv_core::swap_key!(WorkOrdersMachineCreateModalKey, "wo-machine-create-modal");
lariv_core::swap_key!(WorkOrdersMachineEditModalKey, "wo-machine-edit-modal");
lariv_core::swap_key!(WorkOrdersMachineDeleteModalKey, "wo-machine-delete-modal");
