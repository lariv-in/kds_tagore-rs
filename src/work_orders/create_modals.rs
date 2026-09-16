//! Typed [`CreateModal`] wiring for Work Orders swap keys.

use super::keys::{
    ComponentCreateModalKey, InvoiceCreateModalKey, MachineCreateModalKey, MachineSelectModalKey,
    MachineSelectTableKey, MaterialCreateModalKey, MaterialRateCreateModalKey,
    MaterialSelectModalKey, MaterialSelectTableKey, ShapeCreateModalKey, ShapeSelectModalKey,
    ShapeSelectTableKey, WorkOrderCreateModalKey,
};
use super::routes::{
    ComponentCreateGetRouteTag, ComponentCreatePostRouteTag, InvoiceCreateGetRouteTag,
    InvoiceCreatePostRouteTag, MachineCreateGetRouteTag, MachineCreatePostRouteTag,
    MaterialCreateGetRouteTag, MaterialCreatePostRouteTag, MaterialRateCreateGetRouteTag,
    MaterialRateCreatePostRouteTag, ShapeCreateGetRouteTag, ShapeCreatePostRouteTag,
    WorkOrderCreateGetRouteTag, WorkOrderCreatePostRouteTag,
};

lariv_rs::impl_create_modal!(
    WorkOrderCreateModalKey,
    WorkOrderCreateGetRouteTag,
    WorkOrderCreatePostRouteTag,
    "wo.WorkOrderCreateForm"
);

lariv_rs::impl_create_modal!(
    ComponentCreateModalKey,
    ComponentCreateGetRouteTag,
    ComponentCreatePostRouteTag,
    "wo.ComponentCreateForm"
);

lariv_rs::impl_create_modal!(
    ShapeCreateModalKey,
    ShapeCreateGetRouteTag,
    ShapeCreatePostRouteTag,
    "wo.ShapeCreateForm"
);

lariv_rs::impl_create_modal!(
    MaterialCreateModalKey,
    MaterialCreateGetRouteTag,
    MaterialCreatePostRouteTag,
    "wo.MaterialCreateForm"
);

lariv_rs::impl_create_modal!(
    MaterialRateCreateModalKey,
    MaterialRateCreateGetRouteTag,
    MaterialRateCreatePostRouteTag,
    "wo.MaterialRateCreateForm"
);

lariv_rs::impl_create_modal!(
    MachineCreateModalKey,
    MachineCreateGetRouteTag,
    MachineCreatePostRouteTag,
    "wo.MachineCreateForm"
);

lariv_rs::impl_create_modal!(
    InvoiceCreateModalKey,
    InvoiceCreateGetRouteTag,
    InvoiceCreatePostRouteTag,
    "wo.InvoiceCreateForm"
);

lariv_rs::impl_picker_modal!(ShapeSelectModalKey, ShapeSelectTableKey);
lariv_rs::impl_picker_modal!(MaterialSelectModalKey, MaterialSelectTableKey);
lariv_rs::impl_picker_modal!(MachineSelectModalKey, MachineSelectTableKey);

