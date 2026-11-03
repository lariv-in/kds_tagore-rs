//! Typed create-modal wiring for the work order list.

use lariv_core::web::CreateModal;

use super::keys::WORK_ORDER_CREATE_MODAL;
use super::routes::{WorkOrderCreateGetRouteTag, WorkOrderCreatePostRouteTag};

pub const WORK_ORDER_CREATE: CreateModal<WorkOrderCreateGetRouteTag, WorkOrderCreatePostRouteTag> =
    CreateModal {
        key: WORK_ORDER_CREATE_MODAL,
        form_name: "wo.WorkOrderCreateForm",
        get: WorkOrderCreateGetRouteTag,
        post: WorkOrderCreatePostRouteTag,
    };
