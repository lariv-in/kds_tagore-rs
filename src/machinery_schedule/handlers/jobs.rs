use axum::{
    extract::{Path, Query},
    http::Uri,
    response::{IntoResponse, Redirect, Response},
};
use chrono::Utc;
use lariv_rs::{
    components::{DEFAULT_PAGE_SIZE, ManyToManyItem, ObjectList, SharedChromeFolder, SlotCtx, SwapKey},
    html_form::HtmlFormBody,
    http::{Cap, RouteQueryBuilder},
    plugins::{
        filesystem::entities::filesystem_node::{Column as VNodeColumn, Entity as VNodeEntity},
        users::middleware::RequireAuth,
    },
    template::RenderAppPane,
    web::{
        Htmx, QueryPage, html_built_page_or_app_layout, html_built_page_with_slots,
        respond_create_modal_done, respond_edit_modal_done,
    },
};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, JoinType, PaginatorTrait,
    QueryFilter, QuerySelect, RelationTrait, TransactionTrait,
};

use crate::machinery_schedule::{
    crumbs::jobs_tab_url,
    entities::{
        completed_job, job::{self, Entity as JobEntity}, job_machine,
        machine::{self, Entity as MachineEntity},
    },
    forms::JobForm,
    handlers::{
        BulkIdsForm, BulkIdsQuery, ModalNameQuery, bulk_delete_message, parse_bulk_ids,
        path_and_query,
    },
    keys::{JobBulkDeleteModalKey, JobCreateModalKey, JobDeleteModalKey, JobEditModalKey, JobHubTableKey},
    logic::{
        clamp_progress, complete_job_if_needed, completed_job_id_for_job, delete_open_job,
        duplicate_job, err_if_job_completed, format_job_duration, load_job_file_ids,
        load_job_machine_ids, move_open_job_order, next_order_after_move, open_job_orders,
        parse_job_duration, parse_job_progress, sync_job_files, sync_job_machines, OrderMove,
    },
    routes::{
        CompletedJobDetailRouteTag, JobBulkDeletePostRouteTag, JobDefaultRouteTag, JobDetailRouteTag,
    },
    scope::{
        apply_completed_job_hub_sort, apply_name_filter, apply_open_job_hub_sort, find_job_scoped,
        find_open_job,
        scope_superuser, sql_job_not_completed,
    },
    state::MachineryScheduleState,
    templates::{
        ConfirmBulkDeletePage, ConfirmDeletePage, JobCreateModalPage, JobDetailPage,
        JobDuplicatedModalPage, JobEditModalPage, JobHubPage, JobRow,
    },
};

#[derive(Debug, serde::Deserialize, Default)]
pub struct HubQuery {
    #[serde(default)]
    pub tab: Option<String>,
    #[serde(default, rename = "Name", alias = "name")]
    pub name: Option<String>,
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub page: QueryPage,
}

fn normalize_tab(tab: Option<&str>) -> &'static str {
    match tab.unwrap_or("jobs") {
        "completed" => "completed",
        _ => "jobs",
    }
}

fn hub_list_path(q: &HubQuery) -> String {
    let mut builder = RouteQueryBuilder::new(JobDefaultRouteTag)
        .query("tab", normalize_tab(q.tab.as_deref()));
    if let Some(name) = q.name.as_deref().filter(|s| !s.is_empty()) {
        builder = builder.query("Name", name);
    }
    if let Some(sort) = q.sort.as_deref().filter(|s| !s.is_empty()) {
        builder = builder.query("sort", sort);
    }
    if q.page.get() > 1 {
        builder = builder.query("page", q.page.get());
    }
    builder.build()
}

pub async fn machine_items_from_ids(
    db: &sea_orm::DatabaseConnection,
    ids: &[i64],
) -> Vec<ManyToManyItem> {
    if ids.is_empty() {
        return Vec::new();
    }
    let machines = MachineEntity::find()
        .filter(machine::Column::Id.is_in(ids.to_vec()))
        .all(db)
        .await
        .unwrap_or_default();
    ids.iter()
        .filter_map(|id| {
            machines
                .iter()
                .find(|m| m.id == *id)
                .map(|m| ManyToManyItem::new(m.id.to_string(), m.name.clone()))
        })
        .collect()
}

pub async fn file_items_from_ids(
    db: &sea_orm::DatabaseConnection,
    ids: &[i64],
) -> Vec<ManyToManyItem> {
    if ids.is_empty() {
        return Vec::new();
    }
    let nodes = VNodeEntity::find()
        .filter(VNodeColumn::Id.is_in(ids.to_vec()))
        .all(db)
        .await
        .unwrap_or_default();
    ids.iter()
        .filter_map(|id| {
            nodes
                .iter()
                .find(|n| n.id == *id)
                .map(|n| ManyToManyItem::new(n.id.to_string(), n.name.clone()))
        })
        .collect()
}

async fn machine_counts_by_job_id(
    db: &sea_orm::DatabaseConnection,
    job_ids: &[i64],
) -> std::collections::HashMap<i64, usize> {
    if job_ids.is_empty() {
        return std::collections::HashMap::new();
    }
    let links = job_machine::Entity::find()
        .filter(job_machine::Column::JobId.is_in(job_ids.to_vec()))
        .all(db)
        .await
        .unwrap_or_default();
    let mut counts = std::collections::HashMap::new();
    for link in links {
        *counts.entry(link.job_id).or_insert(0) += 1;
    }
    counts
}

async fn query_jobs_tab(
    db: &sea_orm::DatabaseConnection,
    q: &HubQuery,
    auth: &lariv_rs::plugins::users::state::AuthContext,
) -> ObjectList<JobRow> {
    let tab = normalize_tab(q.tab.as_deref());
    let page = q.page.get();
    let page_size = DEFAULT_PAGE_SIZE;
    match tab {
        "completed" => {
            let mut query = completed_job::Entity::find()
                .join(JoinType::InnerJoin, completed_job::Relation::Job.def());
            query = apply_name_filter(query, job::Column::Name, q.name.as_deref());
            query = scope_superuser(query, auth);
            query = apply_completed_job_hub_sort(query, q.sort.as_deref());
            let paginator = query.paginate(db, page_size as u64);
            let total = paginator.num_items().await.unwrap_or(0);
            let models = paginator
                .fetch_page((page as u64).saturating_sub(1))
                .await
                .unwrap_or_default();
            let job_ids: Vec<i64> = models.iter().map(|c| c.job_id).collect();
            let machine_counts = machine_counts_by_job_id(db, &job_ids).await;
            let mut rows = Vec::with_capacity(models.len());
            for completed in models {
                let Some(job) = JobEntity::find_by_id(completed.job_id)
                    .one(db)
                    .await
                    .ok()
                    .flatten()
                else {
                    continue;
                };
                let machine_count = machine_counts.get(&job.id).copied().unwrap_or(0);
                rows.push(JobRow {
                    id: completed.id,
                    name: job.name,
                    duration: format_job_duration(job.duration),
                    progress: job.progress,
                    order: job.order,
                    machine_count,
                    extra: auth.format_datetime(completed.completed_at).into_string(),
                    detail_href: CompletedJobDetailRouteTag::new(completed.id).url(),
                    can_move_up: false,
                    can_move_down: false,
                });
            }
            ObjectList::from_page(rows, page, page_size, total)
        }
        _ => {
            let mut query = JobEntity::find().filter(sql_job_not_completed());
            query = apply_name_filter(query, job::Column::Name, q.name.as_deref());
            query = scope_superuser(query, auth);
            query = apply_open_job_hub_sort(query, q.sort.as_deref());
            let paginator = query.paginate(db, page_size as u64);
            let total = paginator.num_items().await.unwrap_or(0);
            let models = paginator
                .fetch_page((page as u64).saturating_sub(1))
                .await
                .unwrap_or_default();
            let job_ids: Vec<i64> = models.iter().map(|j| j.id).collect();
            let machine_counts = machine_counts_by_job_id(db, &job_ids).await;
            let orders = open_job_orders(db).await;
            let rows = models
                .into_iter()
                .map(|j| {
                    let machine_count = machine_counts.get(&j.id).copied().unwrap_or(0);
                    JobRow {
                        id: j.id,
                        name: j.name,
                        duration: format_job_duration(j.duration),
                        progress: j.progress,
                        order: j.order,
                        machine_count,
                        extra: String::new(),
                        detail_href: JobDetailRouteTag::new(j.id).url(),
                        can_move_up: next_order_after_move(j.order, &orders, OrderMove::Up)
                            .is_some(),
                        can_move_down: next_order_after_move(j.order, &orders, OrderMove::Down)
                            .is_some(),
                    }
                })
                .collect();
            ObjectList::from_page(rows, page, page_size, total)
        }
    }
}

pub async fn hub(
    Cap(state): Cap<MachineryScheduleState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<HubQuery>,
) -> maud::Markup {
    let tab = normalize_tab(q.tab.as_deref()).to_string();
    let jobs = query_jobs_tab(&state.db, &q, &ctx).await;
    let page = JobHubPage {
        jobs,
        tab,
        filter_name: q.name.clone().unwrap_or_default(),
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        can_edit: ctx.user.is_superuser,
    };
    let slot_ctx = SlotCtx::from_auth(&ctx);
    if htmx.targets::<JobHubTableKey>() {
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

async fn respond_job_order_move(
    state: MachineryScheduleState,
    ctx: lariv_rs::plugins::users::state::AuthContext,
    htmx: Htmx,
    q: HubQuery,
    id: i64,
    dir: OrderMove,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to(&JobDefaultRouteTag.url()).into_response();
    }
    if let Err(e) = move_open_job_order(&state.db, id, &ctx, dir).await {
        tracing::warn!(error = %e, job_id = id, "move job order");
    }
    let jobs = query_jobs_tab(&state.db, &q, &ctx).await;
    let page = JobHubPage {
        jobs,
        tab: normalize_tab(q.tab.as_deref()).to_string(),
        filter_name: q.name.clone().unwrap_or_default(),
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: hub_list_path(&q),
        can_edit: ctx.user.is_superuser,
    };
    if htmx.request {
        return page.render_table().into_response();
    }
    Redirect::to(&hub_list_path(&q)).into_response()
}

pub async fn move_up_post(
    Cap(state): Cap<MachineryScheduleState>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<HubQuery>,
    Path(id): Path<i64>,
) -> Response {
    respond_job_order_move(state, ctx, htmx, q, id, OrderMove::Up).await
}

pub async fn move_down_post(
    Cap(state): Cap<MachineryScheduleState>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<HubQuery>,
    Path(id): Path<i64>,
) -> Response {
    respond_job_order_move(state, ctx, htmx, q, id, OrderMove::Down).await
}

async fn job_detail_links(
    db: &sea_orm::DatabaseConnection,
    job_id: i64,
) -> (Vec<(i64, String)>, Vec<(i64, String)>) {
    let machine_ids = load_job_machine_ids(db, job_id).await;
    let file_ids = load_job_file_ids(db, job_id).await;
    let machines = machine_items_from_ids(db, &machine_ids)
        .await
        .into_iter()
        .filter_map(|item| item.key.parse().ok().map(|id| (id, item.value)))
        .collect();
    let files = file_items_from_ids(db, &file_ids)
        .await
        .into_iter()
        .filter_map(|item| item.key.parse().ok().map(|id| (id, item.value)))
        .collect();
    (machines, files)
}

pub async fn detail(
    Cap(state): Cap<MachineryScheduleState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(job) = find_job_scoped(&state.db, id, &ctx).await else {
        return Redirect::to(&JobDefaultRouteTag.url()).into_response();
    };
    if let Some(completed_id) = completed_job_id_for_job(&state.db, job.id).await {
        return Redirect::to(&CompletedJobDetailRouteTag::new(completed_id).url()).into_response();
    }
    let (machines, files) = job_detail_links(&state.db, job.id).await;
    let page = JobDetailPage {
        id: job.id,
        name: job.name,
        duration: format_job_duration(job.duration),
        progress: job.progress,
        order: job.order,
        remarks: job.remarks,
        machines,
        files,
        can_edit: ctx.user.is_superuser,
        error: String::new(),
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

fn empty_job_form_page(q: &ModalNameQuery) -> JobCreateModalPage {
    JobCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        target_input: q.target_input(),
        name: String::new(),
        machines: Vec::new(),
        duration: String::new(),
        files: Vec::new(),
        order: 0,
        remarks: String::new(),
        progress: 0,
        error: String::new(),
    }
}

pub async fn create_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
) -> maud::Markup {
    if !ctx.user.is_superuser {
        return maud::html! { div class="alert alert-error" { "Forbidden" } };
    }
    html_built_page_with_slots(&empty_job_form_page(&q), &chrome, &SlotCtx::from_auth(&ctx))
}

pub async fn create_post(
    Cap(state): Cap<MachineryScheduleState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<JobForm>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to(&JobDefaultRouteTag.url()).into_response();
    }
    let machines = machine_items_from_ids(&state.db, &form.machines).await;
    let files = file_items_from_ids(&state.db, &form.files).await;
    let render_error = |error: String| {
        let page = JobCreateModalPage {
            form_name: q.form_name(),
            refresh_table: q.refresh_table(),
            target_input: q.target_input(),
            name: form.name.clone(),
            machines: machines.clone(),
            duration: form.duration.clone(),
            files: files.clone(),
            order: form.order,
            remarks: form.remarks.clone(),
            progress: clamp_progress(form.progress),
            error,
        };
        html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
    };
    let name = form.name.trim().to_string();
    if name.is_empty() {
        return render_error("Name is required".into());
    }
    let duration = match parse_job_duration(&form.duration) {
        Ok(duration) => duration,
        Err(e) => return render_error(e),
    };
    let progress = match parse_job_progress(form.progress) {
        Ok(progress) => progress,
        Err(e) => return render_error(e),
    };
    let now = Utc::now();
    let txn = match state.db.begin().await {
        Ok(txn) => txn,
        Err(e) => return render_error(e.to_string()),
    };
    let saved = match (job::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        name: Set(name),
        duration: Set(duration),
        progress: Set(progress),
        order: Set(form.order),
        remarks: Set(form.remarks.trim().to_string()),
    })
    .insert(&txn)
    .await
    {
        Ok(saved) => saved,
        Err(e) => return render_error(e.to_string()),
    };
    if let Err(e) = sync_job_machines(&txn, saved.id, &form.machines).await {
        return render_error(e);
    }
    if let Err(e) = sync_job_files(&txn, saved.id, &form.files).await {
        return render_error(e);
    }
    let completed = match complete_job_if_needed(&txn, saved.id, progress).await {
        Ok(completed) => completed,
        Err(e) => return render_error(e),
    };
    match txn.commit().await {
        Ok(()) => {
            let dest = match completed {
                Some(c) => CompletedJobDetailRouteTag::new(c.id).url(),
                None => JobDetailRouteTag::new(saved.id).url(),
            };
            respond_create_modal_done::<JobCreateModalKey>(&htmx, &q.refresh_table(), &dest)
        }
        Err(e) => render_error(e.to_string()),
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
        return Redirect::to(&JobDefaultRouteTag.url()).into_response();
    }
    let Some(job) = find_open_job(&state.db, id, &ctx).await else {
        return Redirect::to(&JobDefaultRouteTag.url()).into_response();
    };
    let machine_ids = load_job_machine_ids(&state.db, job.id).await;
    let file_ids = load_job_file_ids(&state.db, job.id).await;
    let page = JobEditModalPage {
        id: job.id,
        form_name: q.form_name(),
        name: job.name,
        machines: machine_items_from_ids(&state.db, &machine_ids).await,
        duration: format_job_duration(job.duration),
        files: file_items_from_ids(&state.db, &file_ids).await,
        order: job.order,
        remarks: job.remarks,
        progress: job.progress,
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
    HtmlFormBody(form): HtmlFormBody<JobForm>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to(&JobDefaultRouteTag.url()).into_response();
    }
    let Some(existing) = find_open_job(&state.db, id, &ctx).await else {
        return Redirect::to(&JobDefaultRouteTag.url()).into_response();
    };
    let machines = machine_items_from_ids(&state.db, &form.machines).await;
    let files = file_items_from_ids(&state.db, &form.files).await;
    let render_error = |error: String| {
        let page = JobEditModalPage {
            id,
            form_name: q.form_name(),
            name: form.name.clone(),
            machines: machines.clone(),
            duration: form.duration.clone(),
            files: files.clone(),
            order: form.order,
            remarks: form.remarks.clone(),
            progress: clamp_progress(form.progress),
            error,
        };
        html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
    };
    if let Err(e) = err_if_job_completed(&state.db, id).await {
        return render_error(e);
    }
    let name = form.name.trim().to_string();
    if name.is_empty() {
        return render_error("Name is required".into());
    }
    let duration = match parse_job_duration(&form.duration) {
        Ok(duration) => duration,
        Err(e) => return render_error(e),
    };
    let progress = match parse_job_progress(form.progress) {
        Ok(progress) => progress,
        Err(e) => return render_error(e),
    };
    let now = Utc::now();
    let txn = match state.db.begin().await {
        Ok(txn) => txn,
        Err(e) => return render_error(e.to_string()),
    };
    let mut am: job::ActiveModel = existing.into();
    am.updated_at = Set(Some(now));
    am.name = Set(name);
    am.duration = Set(duration);
    am.progress = Set(progress);
    am.order = Set(form.order);
    am.remarks = Set(form.remarks.trim().to_string());
    if let Err(e) = am.update(&txn).await {
        return render_error(e.to_string());
    }
    if let Err(e) = sync_job_machines(&txn, id, &form.machines).await {
        return render_error(e);
    }
    if let Err(e) = sync_job_files(&txn, id, &form.files).await {
        return render_error(e);
    }
    let completed = match complete_job_if_needed(&txn, id, progress).await {
        Ok(completed) => completed,
        Err(e) => return render_error(e),
    };
    match txn.commit().await {
        Ok(()) => {
            let dest = match completed {
                Some(c) => CompletedJobDetailRouteTag::new(c.id).url(),
                None => JobDetailRouteTag::new(id).url(),
            };
            respond_edit_modal_done::<JobEditModalKey>(&htmx, &dest)
        }
        Err(e) => render_error(e.to_string()),
    }
}

pub async fn delete_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
    Path(id): Path<i64>,
) -> maud::Markup {
    let page = ConfirmDeletePage {
        modal_uid: JobDeleteModalKey::ID.to_string(),
        message: "Are you sure you want to delete this job?".into(),
        form_name: q
            .name
            .clone()
            .unwrap_or_else(|| "kds_ms.JobDeleteForm".into()),
        post_url: crate::machinery_schedule::routes::JobDeletePostRouteTag::new(id).url(),
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
        return Redirect::to(&JobDefaultRouteTag.url()).into_response();
    }
    match delete_open_job(&state.db, id, &ctx).await {
        Ok(()) => htmx.redirect(&JobDefaultRouteTag.url()),
        Err(e) => {
            tracing::error!(error = %e, id, "failed to delete job");
            let page = ConfirmDeletePage {
                modal_uid: JobDeleteModalKey::ID.to_string(),
                message: "Are you sure you want to delete this job?".into(),
                form_name: "kds_ms.JobDeleteForm".into(),
                post_url: crate::machinery_schedule::routes::JobDeletePostRouteTag::new(id).url(),
                error: e,
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

pub fn respond_duplicated_job_modal(
    chrome: &SharedChromeFolder,
    ctx: &lariv_rs::plugins::users::state::AuthContext,
    result: Result<crate::machinery_schedule::entities::job::Model, String>,
) -> Response {
    let page = match result {
        Ok(job) => JobDuplicatedModalPage {
            job_id: job.id,
            name: job.name,
            error: String::new(),
        },
        Err(error) => JobDuplicatedModalPage {
            job_id: 0,
            name: String::new(),
            error,
        },
    };
    html_built_page_with_slots(&page, chrome, &SlotCtx::from_auth(ctx)).into_response()
}

pub async fn duplicate_post(
    Cap(state): Cap<MachineryScheduleState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to(&JobDefaultRouteTag.url()).into_response();
    }
    match duplicate_job(&state.db, id, &ctx).await {
        Ok(job) => {
            if !htmx.request {
                return Redirect::to(&JobDetailRouteTag::new(job.id).url()).into_response();
            }
            respond_duplicated_job_modal(&chrome, &ctx, Ok(job))
        }
        Err(e) => respond_duplicated_job_modal(&chrome, &ctx, Err(e)),
    }
}

fn job_bulk_delete_page(ids: &[i64], error: String) -> ConfirmBulkDeletePage {
    ConfirmBulkDeletePage {
        modal_uid: JobBulkDeleteModalKey::ID.to_string(),
        message: if ids.is_empty() {
            "Select at least one job to delete.".into()
        } else {
            bulk_delete_message("jobs", ids.len())
        },
        ids: ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(","),
        post_url: JobBulkDeletePostRouteTag.url(),
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
        &job_bulk_delete_page(&ids, String::new()),
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
        return Redirect::to(&jobs_tab_url("jobs")).into_response();
    }
    let ids = parse_bulk_ids(&form.ids);
    if ids.is_empty() {
        return html_built_page_with_slots(
            &job_bulk_delete_page(&ids, String::new()),
            &chrome,
            &SlotCtx::from_auth(&ctx),
        )
        .into_response();
    }
    for id in &ids {
        if let Err(e) = delete_open_job(&state.db, *id, &ctx).await {
            tracing::error!(error = %e, id, "failed to bulk-delete job");
            return html_built_page_with_slots(
                &job_bulk_delete_page(&ids, format!("Failed to delete job #{id}: {e}")),
                &chrome,
                &SlotCtx::from_auth(&ctx),
            )
            .into_response();
        }
    }
    htmx.redirect(&jobs_tab_url("jobs"))
}

pub async fn bulk_duplicate_post(
    Cap(state): Cap<MachineryScheduleState>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<BulkIdsQuery>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to(&jobs_tab_url("jobs")).into_response();
    }
    for id in parse_bulk_ids(q.ids.as_deref().unwrap_or("")) {
        let _ = duplicate_job(&state.db, id, &ctx).await;
    }
    htmx.redirect(&jobs_tab_url("jobs"))
}
