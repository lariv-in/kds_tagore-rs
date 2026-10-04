use super::{
    handlers,
    keys::{
        CompletedJobBulkDeleteModalKey, CompletedJobDeleteModalKey, JobBulkDeleteModalKey,
        JobDeleteModalKey, JobHubTableKey, MachineDeleteModalKey, MachineJobsTableKey,
        MachineSelectModalKey, MachineSelectTableKey, MachineTableKey,
    },
};

use crate::hr_role::Hr;

/// Machinery Schedule routes. Allowlist is [`Hr`]; superuser always passes.
pub struct MachineryScheduleAccess;

lariv_rs::define_plugin_routes! {
    plugin: MachineryScheduleTag;
    prefix: "/dashboard";
    routes: [
        get JobDefaultRouteTag, "/machinery-schedule", handlers::jobs::hub, fragment(JobHubTableKey), authorize(MachineryScheduleAccess, [Hr]);
        get JobCreateGetRouteTag, "/machinery-schedule/jobs/create", handlers::jobs::create_get, modal, authorize(MachineryScheduleAccess, [Hr]);
        post JobCreatePostRouteTag, "/machinery-schedule/jobs/create", handlers::jobs::create_post, authorize(MachineryScheduleAccess, [Hr]);
        get JobBulkDeleteGetRouteTag, "/machinery-schedule/jobs/bulk-delete", handlers::jobs::bulk_delete_get, modal, authorize(MachineryScheduleAccess, [Hr]);
        post JobBulkDeletePostRouteTag, "/machinery-schedule/jobs/bulk-delete", bare handlers::jobs::bulk_delete_post, fragment(JobBulkDeleteModalKey), authorize(MachineryScheduleAccess, [Hr]);
        post JobBulkDuplicatePostRouteTag, "/machinery-schedule/jobs/bulk-duplicate", bare handlers::jobs::bulk_duplicate_post, redirect, authorize(MachineryScheduleAccess, [Hr]);
        get JobDetailRouteTag, "/machinery-schedule/jobs/{id}", handlers::jobs::detail, authorize(MachineryScheduleAccess, [Hr]);
        get JobEditGetRouteTag, "/machinery-schedule/jobs/{id}/edit", handlers::jobs::edit_get, modal, authorize(MachineryScheduleAccess, [Hr]);
        post JobEditPostRouteTag, "/machinery-schedule/jobs/{id}/edit", handlers::jobs::edit_post, authorize(MachineryScheduleAccess, [Hr]);
        get JobDeleteGetRouteTag, "/machinery-schedule/jobs/{id}/delete", handlers::jobs::delete_get, modal, authorize(MachineryScheduleAccess, [Hr]);
        post JobDeletePostRouteTag, "/machinery-schedule/jobs/{id}/delete", bare handlers::jobs::delete_post, fragment(JobDeleteModalKey), authorize(MachineryScheduleAccess, [Hr]);
        post JobDuplicatePostRouteTag, "/machinery-schedule/jobs/{id}/duplicate", handlers::jobs::duplicate_post, modal, authorize(MachineryScheduleAccess, [Hr]);
        post JobMoveUpPostRouteTag, "/machinery-schedule/jobs/{id}/move-up", bare handlers::jobs::move_up_post, fragment(JobHubTableKey), authorize(MachineryScheduleAccess, [Hr]);
        post JobMoveDownPostRouteTag, "/machinery-schedule/jobs/{id}/move-down", bare handlers::jobs::move_down_post, fragment(JobHubTableKey), authorize(MachineryScheduleAccess, [Hr]);

        get CompletedJobBulkDeleteGetRouteTag, "/machinery-schedule/completed/bulk-delete", handlers::completed_jobs::bulk_delete_get, modal, authorize(MachineryScheduleAccess, [Hr]);
        post CompletedJobBulkDeletePostRouteTag, "/machinery-schedule/completed/bulk-delete", bare handlers::completed_jobs::bulk_delete_post, fragment(CompletedJobBulkDeleteModalKey), authorize(MachineryScheduleAccess, [Hr]);
        post CompletedJobBulkNewJobPostRouteTag, "/machinery-schedule/completed/bulk-new-job", bare handlers::completed_jobs::bulk_new_job_post, redirect, authorize(MachineryScheduleAccess, [Hr]);
        get CompletedJobDetailRouteTag, "/machinery-schedule/completed/{id}", handlers::completed_jobs::detail, authorize(MachineryScheduleAccess, [Hr]);
        post CompletedJobNewJobPostRouteTag, "/machinery-schedule/completed/{id}/new-job", handlers::completed_jobs::new_job_post, modal, authorize(MachineryScheduleAccess, [Hr]);
        get CompletedJobDeleteGetRouteTag, "/machinery-schedule/completed/{id}/delete", handlers::completed_jobs::delete_get, modal, authorize(MachineryScheduleAccess, [Hr]);
        post CompletedJobDeletePostRouteTag, "/machinery-schedule/completed/{id}/delete", bare handlers::completed_jobs::delete_post, fragment(CompletedJobDeleteModalKey), authorize(MachineryScheduleAccess, [Hr]);

        get MachineDefaultRouteTag, "/machinery-schedule/machines", handlers::machines::list, fragment(MachineTableKey), authorize(MachineryScheduleAccess, [Hr]);
        get MachineCreateGetRouteTag, "/machinery-schedule/machines/create", handlers::machines::create_get, modal, authorize(MachineryScheduleAccess, [Hr]);
        post MachineCreatePostRouteTag, "/machinery-schedule/machines/create", handlers::machines::create_post, authorize(MachineryScheduleAccess, [Hr]);
        get MachineDetailRouteTag, "/machinery-schedule/machines/{id}", handlers::machines::detail, fragment(MachineJobsTableKey), authorize(MachineryScheduleAccess, [Hr]);
        get MachineEditGetRouteTag, "/machinery-schedule/machines/{id}/edit", handlers::machines::edit_get, modal, authorize(MachineryScheduleAccess, [Hr]);
        post MachineEditPostRouteTag, "/machinery-schedule/machines/{id}/edit", handlers::machines::edit_post, authorize(MachineryScheduleAccess, [Hr]);
        get MachineDeleteGetRouteTag, "/machinery-schedule/machines/{id}/delete", handlers::machines::delete_get, modal, authorize(MachineryScheduleAccess, [Hr]);
        post MachineDeletePostRouteTag, "/machinery-schedule/machines/{id}/delete", bare handlers::machines::delete_post, fragment(MachineDeleteModalKey), authorize(MachineryScheduleAccess, [Hr]);
        get MachineFkSelectRouteTag, "/machinery-schedule/machines/pick", handlers::machines::select, fk_select(MachineSelectTableKey, MachineSelectModalKey), authorize(MachineryScheduleAccess, [Hr]);
    ]
}
