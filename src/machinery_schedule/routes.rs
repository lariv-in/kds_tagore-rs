use super::{
    handlers,
    keys::{
        CompletedJobBulkDeleteModalKey, CompletedJobDeleteModalKey, JobBulkDeleteModalKey,
        JobDeleteModalKey, JobHubTableKey, MachineDeleteModalKey, MachineJobsTableKey,
        MachineSelectModalKey, MachineSelectTableKey, MachineTableKey,
    },
};

lariv_rs::define_plugin_routes! {
    plugin: MachineryScheduleTag;
    routes: [
        get JobDefaultRouteTag, "/machinery-schedule", handlers::jobs::hub, fragment(JobHubTableKey);
        get JobCreateGetRouteTag, "/machinery-schedule/jobs/create", handlers::jobs::create_get, modal;
        post JobCreatePostRouteTag, "/machinery-schedule/jobs/create", handlers::jobs::create_post;
        get JobBulkDeleteGetRouteTag, "/machinery-schedule/jobs/bulk-delete", handlers::jobs::bulk_delete_get, modal;
        post JobBulkDeletePostRouteTag, "/machinery-schedule/jobs/bulk-delete", bare handlers::jobs::bulk_delete_post, fragment(JobBulkDeleteModalKey);
        post JobBulkDuplicatePostRouteTag, "/machinery-schedule/jobs/bulk-duplicate", bare handlers::jobs::bulk_duplicate_post, redirect;
        get JobDetailRouteTag, "/machinery-schedule/jobs/{id}", handlers::jobs::detail;
        get JobEditGetRouteTag, "/machinery-schedule/jobs/{id}/edit", handlers::jobs::edit_get, modal;
        post JobEditPostRouteTag, "/machinery-schedule/jobs/{id}/edit", handlers::jobs::edit_post;
        get JobDeleteGetRouteTag, "/machinery-schedule/jobs/{id}/delete", handlers::jobs::delete_get, modal;
        post JobDeletePostRouteTag, "/machinery-schedule/jobs/{id}/delete", bare handlers::jobs::delete_post, fragment(JobDeleteModalKey);
        post JobDuplicatePostRouteTag, "/machinery-schedule/jobs/{id}/duplicate", handlers::jobs::duplicate_post, modal;
        post JobMoveUpPostRouteTag, "/machinery-schedule/jobs/{id}/move-up", bare handlers::jobs::move_up_post, fragment(JobHubTableKey);
        post JobMoveDownPostRouteTag, "/machinery-schedule/jobs/{id}/move-down", bare handlers::jobs::move_down_post, fragment(JobHubTableKey);

        get CompletedJobBulkDeleteGetRouteTag, "/machinery-schedule/completed/bulk-delete", handlers::completed_jobs::bulk_delete_get, modal;
        post CompletedJobBulkDeletePostRouteTag, "/machinery-schedule/completed/bulk-delete", bare handlers::completed_jobs::bulk_delete_post, fragment(CompletedJobBulkDeleteModalKey);
        post CompletedJobBulkNewJobPostRouteTag, "/machinery-schedule/completed/bulk-new-job", bare handlers::completed_jobs::bulk_new_job_post, redirect;
        get CompletedJobDetailRouteTag, "/machinery-schedule/completed/{id}", handlers::completed_jobs::detail;
        post CompletedJobNewJobPostRouteTag, "/machinery-schedule/completed/{id}/new-job", handlers::completed_jobs::new_job_post, modal;
        get CompletedJobDeleteGetRouteTag, "/machinery-schedule/completed/{id}/delete", handlers::completed_jobs::delete_get, modal;
        post CompletedJobDeletePostRouteTag, "/machinery-schedule/completed/{id}/delete", bare handlers::completed_jobs::delete_post, fragment(CompletedJobDeleteModalKey);

        get MachineDefaultRouteTag, "/machinery-schedule/machines", handlers::machines::list, fragment(MachineTableKey);
        get MachineCreateGetRouteTag, "/machinery-schedule/machines/create", handlers::machines::create_get, modal;
        post MachineCreatePostRouteTag, "/machinery-schedule/machines/create", handlers::machines::create_post;
        get MachineDetailRouteTag, "/machinery-schedule/machines/{id}", handlers::machines::detail, fragment(MachineJobsTableKey);
        get MachineEditGetRouteTag, "/machinery-schedule/machines/{id}/edit", handlers::machines::edit_get, modal;
        post MachineEditPostRouteTag, "/machinery-schedule/machines/{id}/edit", handlers::machines::edit_post;
        get MachineDeleteGetRouteTag, "/machinery-schedule/machines/{id}/delete", handlers::machines::delete_get, modal;
        post MachineDeletePostRouteTag, "/machinery-schedule/machines/{id}/delete", bare handlers::machines::delete_post, fragment(MachineDeleteModalKey);
        get MachineFkSelectRouteTag, "/machinery-schedule/machines/pick", handlers::machines::select, fk_select(MachineSelectTableKey, MachineSelectModalKey);
    ]
}
