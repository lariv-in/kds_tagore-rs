lariv_rs::swap_key!(DraftWorkOrderTableKey, "wo-draft-order-table");
lariv_rs::swap_key!(DraftWorkOrderCreateModalKey, "wo-draft-order-create-modal");
lariv_rs::swap_key!(DraftWorkOrderEditModalKey, "wo-draft-order-edit-modal");
lariv_rs::swap_key!(DraftWorkOrderDeleteModalKey, "wo-draft-order-delete-modal");
lariv_rs::swap_key!(DraftWorkOrderSelectTableKey, "wo-draft-order-select-table");
lariv_rs::swap_key!(DraftWorkOrderSelectModalKey, "wo-draft-order-select-modal");
lariv_rs::swap_key!(DraftWorkOrderMaterialLinesTableKey, "wo-draft-order-material-lines-table");
lariv_rs::swap_key!(DraftWorkOrderMaterialLineEditModalKey, "wo-draft-order-material-line-edit-modal");
lariv_rs::swap_key!(DraftWorkOrderMaterialLineDeleteModalKey, "wo-draft-order-material-line-delete-modal");

lariv_rs::swap_key!(DraftWorkOrderMachineLinesTableKey, "wo-draft-order-machine-lines-table");
lariv_rs::swap_key!(DraftWorkOrderMachineLineEditModalKey, "wo-draft-order-machine-line-edit-modal");
lariv_rs::swap_key!(DraftWorkOrderMachineLineDeleteModalKey, "wo-draft-order-machine-line-delete-modal");
lariv_rs::swap_key!(MachineSelectTableKey, "wo-machine-select-table");
lariv_rs::swap_key!(MachineSelectModalKey, "wo-machine-select-modal");

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

lariv_rs::swap_key!(ComponentTableKey, "wo-component-table");
lariv_rs::swap_key!(ComponentCreateModalKey, "wo-component-create-modal");
lariv_rs::swap_key!(ComponentEditModalKey, "wo-component-edit-modal");
lariv_rs::swap_key!(ComponentDeleteModalKey, "wo-component-delete-modal");
lariv_rs::swap_key!(ComponentSelectTableKey, "wo-component-select-table");
lariv_rs::swap_key!(ComponentSelectModalKey, "wo-component-select-modal");

lariv_rs::swap_key!(ShapeTableKey, "wo-shape-table");
lariv_rs::swap_key!(ShapeCreateModalKey, "wo-shape-create-modal");
lariv_rs::swap_key!(ShapeEditModalKey, "wo-shape-edit-modal");
lariv_rs::swap_key!(ShapeDeleteModalKey, "wo-shape-delete-modal");
lariv_rs::swap_key!(ShapeSelectTableKey, "wo-shape-select-table");
lariv_rs::swap_key!(ShapeSelectModalKey, "wo-shape-select-modal");

lariv_rs::swap_key!(MaterialTableKey, "wo-material-table");
lariv_rs::swap_key!(MaterialCreateModalKey, "wo-material-create-modal");
lariv_rs::swap_key!(MaterialEditModalKey, "wo-material-edit-modal");
lariv_rs::swap_key!(MaterialDeleteModalKey, "wo-material-delete-modal");
lariv_rs::swap_key!(MaterialSelectTableKey, "wo-material-select-table");
lariv_rs::swap_key!(MaterialSelectModalKey, "wo-material-select-modal");

lariv_rs::swap_key!(MaterialRateTableKey, "wo-rate-table");
lariv_rs::swap_key!(MaterialRateCreateModalKey, "wo-rate-create-modal");
lariv_rs::swap_key!(MaterialRateDeleteModalKey, "wo-rate-delete-modal");

lariv_rs::swap_key!(MachineTableKey, "wo-machine-table");
lariv_rs::swap_key!(MachineCreateModalKey, "wo-machine-create-modal");
lariv_rs::swap_key!(MachineEditModalKey, "wo-machine-edit-modal");
lariv_rs::swap_key!(MachineDeleteModalKey, "wo-machine-delete-modal");

lariv_rs::swap_key!(InvoiceTableKey, "wo-invoice-table");
lariv_rs::swap_key!(InvoiceCreateModalKey, "wo-invoice-create-modal");
lariv_rs::swap_key!(InvoiceEditModalKey, "wo-invoice-edit-modal");
lariv_rs::swap_key!(InvoiceDeleteModalKey, "wo-invoice-delete-modal");
