//! Typed create-modal wiring for the work order list.

use super::keys::WorkOrderCreateModalKey;
use super::routes::{WorkOrderCreateGetRouteTag, WorkOrderCreatePostRouteTag};

lariv_core::impl_create_modal!(
    WorkOrderCreateModalKey,
    WorkOrderCreateGetRouteTag,
    WorkOrderCreatePostRouteTag,
    "wo.WorkOrderCreateForm"
);
