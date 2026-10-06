//! Typed [`CreateModal`] wiring for KDS Quotations swap keys.

use super::keys::{
    ComponentCreateModalKey, InvoiceCreateModalKey, WorkOrderCreateModalKey,
    WorkOrdersMachineCreateModalKey,
};
use super::routes::{
    ComponentCreateGetRouteTag, ComponentCreatePostRouteTag, InvoiceCreateGetRouteTag,
    InvoiceCreatePostRouteTag, WorkOrderCreateGetRouteTag, WorkOrderCreatePostRouteTag,
    WorkOrdersMachineCreateGetRouteTag, WorkOrdersMachineCreatePostRouteTag,
};

lariv_core::impl_create_modal!(
    WorkOrderCreateModalKey,
    WorkOrderCreateGetRouteTag,
    WorkOrderCreatePostRouteTag,
    "wo.WorkOrderCreateForm"
);

lariv_core::impl_create_modal!(
    ComponentCreateModalKey,
    ComponentCreateGetRouteTag,
    ComponentCreatePostRouteTag,
    "wo.ComponentCreateForm"
);

lariv_core::impl_create_modal!(
    InvoiceCreateModalKey,
    InvoiceCreateGetRouteTag,
    InvoiceCreatePostRouteTag,
    "wo.InvoiceCreateForm"
);

lariv_core::impl_create_modal!(
    WorkOrdersMachineCreateModalKey,
    WorkOrdersMachineCreateGetRouteTag,
    WorkOrdersMachineCreatePostRouteTag,
    "wo.MachineCreateForm"
);
