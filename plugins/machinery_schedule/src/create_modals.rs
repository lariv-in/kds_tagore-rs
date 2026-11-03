//! [`CreateModal`] wiring for Machinery Schedule swap keys.

use lariv_core::web::CreateModal;

use super::keys::{JOB_CREATE_MODAL, MACHINE_CREATE_MODAL};
use super::routes::{
    JobCreateGetRouteTag, JobCreatePostRouteTag, MachineCreateGetRouteTag,
    MachineCreatePostRouteTag,
};

pub const MACHINE_CREATE: CreateModal<MachineCreateGetRouteTag, MachineCreatePostRouteTag> =
    CreateModal {
        key: MACHINE_CREATE_MODAL,
        form_name: "kds_ms.MachineCreateForm",
        get: MachineCreateGetRouteTag,
        post: MachineCreatePostRouteTag,
    };

pub const JOB_CREATE: CreateModal<JobCreateGetRouteTag, JobCreatePostRouteTag> = CreateModal {
    key: JOB_CREATE_MODAL,
    form_name: "kds_ms.JobCreateForm",
    get: JobCreateGetRouteTag,
    post: JobCreatePostRouteTag,
};
