//! Typed [`CreateModal`] / [`PickerModal`] wiring for Machinery Schedule swap keys.

use super::keys::{
    JobCreateModalKey, MachineCreateModalKey, MachineSelectModalKey, MachineSelectTableKey,
};
use super::routes::{
    JobCreateGetRouteTag, JobCreatePostRouteTag, MachineCreateGetRouteTag,
    MachineCreatePostRouteTag,
};

lariv_rs::impl_create_modal!(
    MachineCreateModalKey,
    MachineCreateGetRouteTag,
    MachineCreatePostRouteTag,
    "kds_ms.MachineCreateForm"
);
lariv_rs::impl_create_modal!(
    JobCreateModalKey,
    JobCreateGetRouteTag,
    JobCreatePostRouteTag,
    "kds_ms.JobCreateForm"
);
lariv_rs::impl_picker_modal!(MachineSelectModalKey, MachineSelectTableKey);
