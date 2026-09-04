use axum::{
    extract::{Path, Query},
    http::Uri,
    response::{IntoResponse, Redirect, Response},
};
use chrono::Utc;
use lariv_rs::{
    components::{DEFAULT_PAGE_SIZE, ObjectList, SharedChromeFolder, SlotCtx, SwapKey},
    html_form::HtmlFormBody,
    http::Cap,
    picker::respond_picker_select,
    plugins::users::middleware::RequireAuth,
    template::RenderAppPane,
    web::{
        Htmx, QueryPage, html_built_page_or_app_layout, html_built_page_with_slots,
        respond_create_modal_done_fk, respond_edit_modal_done,
    },
};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, EntityTrait, PaginatorTrait,
};

use crate::machinery_schedule::{
    entities::machine::{self, Entity as MachineEntity},
    forms::MachineForm,
    handlers::{ModalNameQuery, path_and_query},
    keys::{
        MachineCreateModalKey, MachineDeleteModalKey, MachineEditModalKey, MachineJobsTableKey,
        MachineSelectModalKey, MachineSelectTableKey, MachineTableKey,
    },
    logic::{
        completed_job_id_for_job, format_job_duration, jobs_for_machine, machine_free_on,
        machine_remaining_duration,
    },
    routes::{CompletedJobDetailRouteTag, JobDetailRouteTag, MachineDetailRouteTag},
    scope::{apply_name_filter, apply_name_sort_or_id_desc, find_machine_scoped, scope_superuser},
    state::MachineryScheduleState,
    templates::{
        ConfirmDeletePage, MachineCreateModalPage, MachineDetailPage, MachineEditModalPage,
        MachineJobRow, MachineListPage, MachineRow, MachineSelectPage,
    },
};

#[derive(Debug, serde::Deserialize, Default)]
pub struct MachineListQuery {
    #[serde(default, rename = "Name", alias = "name")]
    pub name: Option<String>,
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub page: QueryPage,
}

#[derive(Debug, serde::Deserialize, Default)]
pub struct MachineSelectQuery {
    #[serde(flatten)]
    pub filter: MachineListQuery,
    #[serde(default)]
    pub target_input: Option<String>,
    #[serde(default)]
    pub multi: Option<String>,
}

fn query_is_multi(raw: Option<&str>) -> bool {
    matches!(raw, Some("1") | Some("true") | Some("True"))
}

async fn load_machine_rows(
    db: &sea_orm::DatabaseConnection,
    q: &MachineListQuery,
    auth: &lariv_rs::plugins::users::state::AuthContext,
    page_size: u32,
) -> ObjectList<MachineRow> {
    let mut query = MachineEntity::find();
    query = apply_name_filter(query, machine::Column::Name, q.name.as_deref());
    query = scope_superuser(query, auth);
    query = apply_name_sort_or_id_desc(
        query,
        machine::Column::Name,
        machine::Column::Id,
        q.sort.as_deref(),
    );
    let page = q.page.get();
    let paginator = query.paginate(db, page_size as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    let rows = models
        .into_iter()
        .map(|m| MachineRow {
            id: m.id,
            name: m.name,
        })
        .collect();
    ObjectList::from_page(rows, page, page_size, total)
}

pub async fn list(
    Cap(state): Cap<MachineryScheduleState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<MachineListQuery>,
) -> maud::Markup {
    let machines = load_machine_rows(&state.db, &q, &ctx, DEFAULT_PAGE_SIZE).await;
    let page = MachineListPage {
        machines,
        filter_name: q.name.clone().unwrap_or_default(),
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        can_edit: ctx.user.is_superuser,
    };
    let slot_ctx = SlotCtx::from_auth(&ctx);
    if htmx.targets::<MachineTableKey>() {
        return page.render_table();
    }
    if htmx.wants_main_content() {
        return page.render_main().into();
    }
    if htmx.wants_app_layout() {
        return page.render_pane().into();
    }
    html_built_page_with_slots(&page, &chrome, &slot_ctx)
}

async fn load_machine_jobs(
    db: &sea_orm::DatabaseConnection,
    machine_id: i64,
) -> Vec<MachineJobRow> {
    let jobs = jobs_for_machine(db, machine_id).await.unwrap_or_default();
    let mut rows = Vec::with_capacity(jobs.len());
    for job in jobs {
        let completed_id = completed_job_id_for_job(db, job.id).await;
        let detail_href = match completed_id {
            Some(id) => CompletedJobDetailRouteTag::new(id).url(),
            None => JobDetailRouteTag::new(job.id).url(),
        };
        rows.push(MachineJobRow {
            id: job.id,
            name: job.name,
            duration: format_job_duration(job.duration),
            progress: job.progress,
            order: job.order,
            detail_href,
        });
    }
    rows
}

pub async fn detail(
    Cap(state): Cap<MachineryScheduleState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(m) = find_machine_scoped(&state.db, id, &ctx).await else {
        return Redirect::to(&crate::machinery_schedule::routes::MachineDefaultRouteTag.url())
            .into_response();
    };
    let jobs = load_machine_jobs(&state.db, m.id).await;
    let remaining = machine_remaining_duration(&state.db, m.id)
        .await
        .unwrap_or_else(|_| chrono::Duration::zero());
    let free_on = ctx
        .format_datetime(machine_free_on(Utc::now(), remaining))
        .into_string();
    let page = MachineDetailPage {
        id: m.id,
        name: m.name,
        can_edit: ctx.user.is_superuser,
        jobs,
        free_on,
    };
    if htmx.targets::<MachineJobsTableKey>() {
        return page.render_jobs_table().into_response();
    }
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn create_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
) -> maud::Markup {
    if !ctx.user.is_superuser {
        return maud::html! { div class="alert alert-error" { "Forbidden" } };
    }
    let page = MachineCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        target_input: q.target_input(),
        name: String::new(),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

pub async fn create_post(
    Cap(state): Cap<MachineryScheduleState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<MachineForm>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to(&crate::machinery_schedule::routes::MachineDefaultRouteTag.url())
            .into_response();
    }
    let name = form.name.trim().to_string();
    if name.is_empty() {
        let page = MachineCreateModalPage {
            form_name: q.form_name(),
            refresh_table: q.refresh_table(),
            target_input: q.target_input(),
            name: form.name,
            error: "Name is required".into(),
        };
        return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
            .into_response();
    }
    let now = Utc::now();
    let model = machine::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        name: Set(name),
    };
    match model.insert(&state.db).await {
        Ok(saved) => respond_create_modal_done_fk::<MachineCreateModalKey>(
            &htmx,
            &q.refresh_table(),
            &MachineDetailRouteTag::new(saved.id).url(),
            saved.id,
            &saved.name,
            &q.target_input(),
        ),
        Err(e) => {
            let page = MachineCreateModalPage {
                form_name: q.form_name(),
                refresh_table: q.refresh_table(),
                target_input: q.target_input(),
                name: form.name,
                error: e.to_string(),
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

pub async fn edit_get(
    Cap(state): Cap<MachineryScheduleState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to(&crate::machinery_schedule::routes::MachineDefaultRouteTag.url())
            .into_response();
    }
    let Some(m) = find_machine_scoped(&state.db, id, &ctx).await else {
        return Redirect::to(&crate::machinery_schedule::routes::MachineDefaultRouteTag.url())
            .into_response();
    };
    let page = MachineEditModalPage {
        id: m.id,
        form_name: q.form_name(),
        name: m.name,
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn edit_post(
    Cap(state): Cap<MachineryScheduleState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<MachineForm>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to(&crate::machinery_schedule::routes::MachineDefaultRouteTag.url())
            .into_response();
    }
    let Some(existing) = find_machine_scoped(&state.db, id, &ctx).await else {
        return Redirect::to(&crate::machinery_schedule::routes::MachineDefaultRouteTag.url())
            .into_response();
    };
    let name = form.name.trim().to_string();
    if name.is_empty() {
        let page = MachineEditModalPage {
            id,
            form_name: q.form_name(),
            name: form.name,
            error: "Name is required".into(),
        };
        return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
            .into_response();
    }
    let now = Utc::now();
    let mut am: machine::ActiveModel = existing.into();
    am.updated_at = Set(Some(now));
    am.name = Set(name);
    match am.update(&state.db).await {
        Ok(_) => respond_edit_modal_done::<MachineEditModalKey>(
            &htmx,
            &MachineDetailRouteTag::new(id).url(),
        ),
        Err(e) => {
            let page = MachineEditModalPage {
                id,
                form_name: q.form_name(),
                name: form.name,
                error: e.to_string(),
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

pub async fn delete_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
    Path(id): Path<i64>,
) -> maud::Markup {
    let page = ConfirmDeletePage {
        modal_uid: MachineDeleteModalKey::ID.to_string(),
        message: "Are you sure you want to delete this machine?".into(),
        form_name: q
            .name
            .clone()
            .unwrap_or_else(|| "kds_ms.MachineDeleteForm".into()),
        post_url: crate::machinery_schedule::routes::MachineDeletePostRouteTag::new(id).url(),
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
    let list_url = crate::machinery_schedule::routes::MachineDefaultRouteTag.url();
    if !ctx.user.is_superuser {
        return Redirect::to(&list_url).into_response();
    }
    match MachineEntity::delete_by_id(id).exec(&state.db).await {
        Ok(_) => htmx.redirect(&list_url),
        Err(e) => {
            tracing::error!(error = %e, id, "failed to delete machine");
            let page = ConfirmDeletePage {
                modal_uid: MachineDeleteModalKey::ID.to_string(),
                message: "Are you sure you want to delete this machine?".into(),
                form_name: "kds_ms.MachineDeleteForm".into(),
                post_url: crate::machinery_schedule::routes::MachineDeletePostRouteTag::new(id)
                    .url(),
                error: e.to_string(),
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

pub async fn select(
    Cap(state): Cap<MachineryScheduleState>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<MachineSelectQuery>,
) -> maud::Markup {
    let machines = load_machine_rows(&state.db, &q.filter, &ctx, DEFAULT_PAGE_SIZE).await;
    let page = MachineSelectPage {
        machines,
        filter_name: q.filter.name.clone().unwrap_or_default(),
        sort: q.filter.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        target_input: q.target_input.clone().unwrap_or_else(|| "Machines".into()),
        can_edit: ctx.user.is_superuser,
        multi: query_is_multi(q.multi.as_deref()),
    };
    respond_picker_select::<MachineSelectTableKey, MachineSelectModalKey, _>(&htmx, &page)
}
