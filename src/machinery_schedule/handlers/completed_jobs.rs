use axum::{
    extract::{Path, Query},
    response::{IntoResponse, Redirect, Response},
};
use lariv_rs::{
    components::{SharedChromeFolder, SlotCtx, SwapKey},
    html_form::HtmlFormBody,
    http::Cap,
    plugins::users::middleware::RequireAuth,
    web::{Htmx, html_built_page_or_app_layout, html_built_page_with_slots},
};
use sea_orm::EntityTrait;

use crate::machinery_schedule::{
    crumbs::jobs_tab_url,
    entities::job::Entity as JobEntity,
    handlers::{
        BulkIdsForm, BulkIdsQuery, ModalNameQuery, bulk_delete_message, parse_bulk_ids,
        jobs::{file_items_from_ids, machine_items_from_ids, respond_duplicated_job_modal},
    },
    keys::{CompletedJobBulkDeleteModalKey, CompletedJobDeleteModalKey},
    logic::{
        delete_completed_job, duplicate_job, format_job_duration, load_job_file_ids,
        load_job_machine_ids,
    },
    routes::{
        CompletedJobBulkDeletePostRouteTag, CompletedJobDeletePostRouteTag, JobDefaultRouteTag,
        JobDetailRouteTag,
    },
    scope::find_completed_job_scoped,
    state::MachineryScheduleState,
    templates::{CompletedJobDetailPage, ConfirmBulkDeletePage, ConfirmDeletePage},
};

async fn completed_detail_page(
    state: &MachineryScheduleState,
    ctx: &lariv_rs::plugins::users::state::AuthContext,
    completed_id: i64,
) -> Option<CompletedJobDetailPage> {
    let completed = find_completed_job_scoped(&state.db, completed_id, ctx).await?;
    let job = JobEntity::find_by_id(completed.job_id)
        .one(&state.db)
        .await
        .ok()
        .flatten()?;
    let machine_ids = load_job_machine_ids(&state.db, job.id).await;
    let file_ids = load_job_file_ids(&state.db, job.id).await;
    let machines = machine_items_from_ids(&state.db, &machine_ids)
        .await
        .into_iter()
        .filter_map(|item| item.key.parse().ok().map(|id| (id, item.value)))
        .collect();
    let files = file_items_from_ids(&state.db, &file_ids)
        .await
        .into_iter()
        .filter_map(|item| item.key.parse().ok().map(|id| (id, item.value)))
        .collect();
    Some(CompletedJobDetailPage {
        id: completed.id,
        name: job.name,
        duration: format_job_duration(job.duration),
        progress: job.progress,
        order: job.order,
        remarks: job.remarks,
        completed_at: ctx.format_datetime(completed.completed_at).into_string(),
        machines,
        files,
        can_edit: ctx.user.is_superuser,
    })
}

pub async fn detail(
    Cap(state): Cap<MachineryScheduleState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(page) = completed_detail_page(&state, &ctx, id).await else {
        return Redirect::to(&JobDefaultRouteTag.url()).into_response();
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn new_job_post(
    Cap(state): Cap<MachineryScheduleState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to(&JobDefaultRouteTag.url()).into_response();
    }
    let Some(completed) = find_completed_job_scoped(&state.db, id, &ctx).await else {
        return Redirect::to(&JobDefaultRouteTag.url()).into_response();
    };
    match duplicate_job(&state.db, completed.job_id, &ctx).await {
        Ok(job) => {
            if !htmx.request {
                return Redirect::to(&JobDetailRouteTag::new(job.id).url()).into_response();
            }
            respond_duplicated_job_modal(&chrome, &ctx, Ok(job))
        }
        Err(e) => respond_duplicated_job_modal(&chrome, &ctx, Err(e)),
    }
}

pub async fn delete_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
    Path(id): Path<i64>,
) -> maud::Markup {
    let page = ConfirmDeletePage {
        modal_uid: CompletedJobDeleteModalKey::ID.to_string(),
        message: "Are you sure you want to delete this completed job?".into(),
        form_name: q
            .name
            .clone()
            .unwrap_or_else(|| "kds_ms.CompletedJobDeleteForm".into()),
        post_url: CompletedJobDeletePostRouteTag::new(id).url(),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

pub async fn delete_post(
    Cap(state): Cap<MachineryScheduleState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to(&jobs_tab_url("completed")).into_response();
    }
    match delete_completed_job(&state.db, id, &ctx).await {
        Ok(()) => htmx.redirect(&jobs_tab_url("completed")),
        Err(e) => {
            tracing::error!(error = %e, id, "failed to delete completed job");
            let page = ConfirmDeletePage {
                modal_uid: CompletedJobDeleteModalKey::ID.to_string(),
                message: "Are you sure you want to delete this completed job?".into(),
                form_name: "kds_ms.CompletedJobDeleteForm".into(),
                post_url: CompletedJobDeletePostRouteTag::new(id).url(),
                error: e,
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

fn completed_bulk_delete_page(ids: &[i64], error: String) -> ConfirmBulkDeletePage {
    ConfirmBulkDeletePage {
        modal_uid: CompletedJobBulkDeleteModalKey::ID.to_string(),
        message: if ids.is_empty() {
            "Select at least one completed job to delete.".into()
        } else {
            bulk_delete_message("completed jobs", ids.len())
        },
        ids: ids
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(","),
        post_url: CompletedJobBulkDeletePostRouteTag.url(),
        error: if ids.is_empty() && error.is_empty() {
            "No jobs selected.".into()
        } else {
            error
        },
        can_submit: !ids.is_empty(),
    }
}

pub async fn bulk_delete_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<BulkIdsQuery>,
) -> maud::Markup {
    let ids = parse_bulk_ids(q.ids.as_deref().unwrap_or(""));
    html_built_page_with_slots(
        &completed_bulk_delete_page(&ids, String::new()),
        &chrome,
        &SlotCtx::from_auth(&ctx),
    )
}

pub async fn bulk_delete_post(
    Cap(state): Cap<MachineryScheduleState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    HtmlFormBody(form): HtmlFormBody<BulkIdsForm>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to(&jobs_tab_url("completed")).into_response();
    }
    let ids = parse_bulk_ids(&form.ids);
    if ids.is_empty() {
        return html_built_page_with_slots(
            &completed_bulk_delete_page(&ids, String::new()),
            &chrome,
            &SlotCtx::from_auth(&ctx),
        )
        .into_response();
    }
    for id in &ids {
        if let Err(e) = delete_completed_job(&state.db, *id, &ctx).await {
            tracing::error!(error = %e, id, "failed to bulk-delete completed job");
            return html_built_page_with_slots(
                &completed_bulk_delete_page(
                    &ids,
                    format!("Failed to delete completed job #{id}: {e}"),
                ),
                &chrome,
                &SlotCtx::from_auth(&ctx),
            )
            .into_response();
        }
    }
    htmx.redirect(&jobs_tab_url("completed"))
}

pub async fn bulk_new_job_post(
    Cap(state): Cap<MachineryScheduleState>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<BulkIdsQuery>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to(&jobs_tab_url("completed")).into_response();
    }
    for id in parse_bulk_ids(q.ids.as_deref().unwrap_or("")) {
        let Some(completed) = find_completed_job_scoped(&state.db, id, &ctx).await else {
            continue;
        };
        let _ = duplicate_job(&state.db, completed.job_id, &ctx).await;
    }
    htmx.redirect(&jobs_tab_url("jobs"))
}
