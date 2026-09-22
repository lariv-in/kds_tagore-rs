//! Typed [`CreateModal`] wiring for KDS Quotations swap keys.

use super::keys::{ComponentCreateModalKey, InvoiceCreateModalKey, WorkOrderCreateModalKey};
use super::routes::{
    ComponentCreateGetRouteTag, ComponentCreatePostRouteTag, InvoiceCreateGetRouteTag,
    InvoiceCreatePostRouteTag, WorkOrderCreateGetRouteTag, WorkOrderCreatePostRouteTag,
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
    InvoiceCreateModalKey,
    InvoiceCreateGetRouteTag,
    InvoiceCreatePostRouteTag,
    "wo.InvoiceCreateForm"
);
