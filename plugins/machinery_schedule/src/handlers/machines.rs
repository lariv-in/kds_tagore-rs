use axum::{
    extract::{Path, Query},
    http::Uri,
    response::{IntoResponse, Redirect, Response},
};
use chrono::Utc;
use lariv_core::{
    components::{ObjectList, SharedChromeFolder, SlotCtx, SwapKey},
    html_form::HtmlFormBody,
    http::Cap,
    picker::respond_picker_select,
    template::RenderAppPane,
    web::{
        Htmx, QueryPage, QueryPageSize, html_built_page_or_app_layout, html_built_page_with_slots,
        respond_create_modal_done_fk, respond_edit_modal_done,
    },
};
use lariv_plugin_users::middleware::RequireAuth;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait, PaginatorTrait, QueryFilter};

use crate::{
    entities::machine::{self, Entity as MachineEntity},
    filters::{machine_fields, prepare},
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
    scope::{
        apply_name_sort_or_id_desc, can_manage, find_machine_scoped, scope_superuser,
        sort_jobs_by_column,
    },
    state::MachineryScheduleState,
    templates::{
        ConfirmDeletePage, MachineCreateModalPage, MachineDetailPage, MachineEditModalPage,
        MachineJobRow, MachineListPage, MachineRow, MachineSelectPage,
    },
};
use lariv_formula::{parse_schema, parse_schema_list, schema_to_entries, schema_to_json};

#[derive(Debug, serde::Deserialize, Default)]
pub struct MachineListQuery {
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub page: QueryPage,
    #[serde(default)]
    pub page_size: QueryPageSize,
}

#[derive(Debug, serde::Deserialize, Default)]
pub struct MachineDetailQuery {
    #[serde(default)]
    pub sort: Option<String>,
}

fn machine_detail_sort(sort: Option<&str>) -> String {
    sort.map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("Order")
        .to_string()
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

fn machine_row(m: &machine::Model) -> MachineRow {
    MachineRow {
        id: m.id,
        name: m.name.clone(),
        formula_label: m.formula_label(),
        variables_json: serde_json::to_string(&m.variables).unwrap_or_else(|_| "{}".into()),
        cost_formula: m.cost_formula.clone(),
    }
}

fn schema_entries_for(m: &machine::Model) -> Vec<String> {
    parse_schema(&m.variables)
        .map(|s| schema_to_entries(&s))
        .unwrap_or_default()
}

fn parse_machine_form(form: &MachineForm) -> Result<(String, String, serde_json::Value), String> {
    let name = form.name.trim().to_string();
    if name.is_empty() {
        return Err("Name is required".into());
    }
    let cost_formula = form.cost_formula.trim().to_string();
    if cost_formula.is_empty() {
        return Err("Cost formula is required".into());
    }
    let schema = parse_schema_list(&form.variables).map_err(|e| e.to_string())?;
    let variables = schema_to_json(&schema);
    let temp = machine::Model {
        id: 0,
        created_at: None,
        updated_at: None,
        name: name.clone(),
        product_id: 0,
        cost_formula: cost_formula.clone(),
        variables: variables.clone(),
    };
    temp.validate_formulas().map_err(|e| e.to_string())?;
    Ok((name, cost_formula, variables))
}

async fn load_machine_rows(
    db: &sea_orm::DatabaseConnection,
    sort: Option<&str>,
    page: u32,
    page_size: u32,
    auth: &lariv_plugin_users::state::AuthContext,
    condition: sea_orm::sea_query::Condition,
) -> ObjectList<MachineRow> {
    let mut query = MachineEntity::find().filter(condition);
    query = scope_superuser(query, auth);
    query = apply_name_sort_or_id_desc(query, machine::Column::Name, machine::Column::Id, sort);
    let paginator = query.paginate(db, page_size as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    let rows = models.into_iter().map(|m| machine_row(&m)).collect();
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
    let prepared = prepare(
        uri.query().unwrap_or(""),
        machine_fields(),
        state.db.get_database_backend(),
    );
    let page_size = q.page_size.get();
    let machines = load_machine_rows(
        &state.db,
        q.sort.as_deref(),
        q.page.get(),
        page_size,
        &ctx,
        prepared.condition,
    )
    .await;
    let page = MachineListPage {
        machines,
        fields: prepared.fields,
        filters: prepared.filters,
        filter_errors: prepared.errors,
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        page_size,
        can_edit: can_manage(&ctx),
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
    sort: Option<&str>,
) -> Vec<MachineJobRow> {
    let mut jobs = jobs_for_machine(db, machine_id).await.unwrap_or_default();
    sort_jobs_by_column(&mut jobs, sort);
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
    uri: Uri,
    Path(id): Path<i64>,
    Query(q): Query<MachineDetailQuery>,
) -> Response {
    let Some(m) = find_machine_scoped(&state.db, id, &ctx).await else {
        return Redirect::to(&crate::routes::MachineDefaultRouteTag.url()).into_response();
    };
    let sort = machine_detail_sort(q.sort.as_deref());
    let jobs = load_machine_jobs(&state.db, m.id, Some(&sort)).await;
    let remaining = machine_remaining_duration(&state.db, m.id)
        .await
        .unwrap_or_else(|_| chrono::Duration::zero());
    let free_on = ctx
        .format_datetime(machine_free_on(Utc::now(), remaining))
        .into_string();
    let page = MachineDetailPage {
        id: m.id,
        formula_label: m.formula_label(),
        variables_label: schema_entries_for(&m).join(", "),
        name: m.name,
        can_edit: can_manage(&ctx),
        jobs,
        free_on,
        sort,
        path_and_query: path_and_query(&uri),
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
    if !can_manage(&ctx) {
        return maud::html! { div class="alert alert-error" { "Forbidden" } };
    }
    let page = MachineCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        target_input: q.target_input(),
        name: String::new(),
        cost_formula: String::new(),
        variables: Vec::new(),
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
    if !can_manage(&ctx) {
        return Redirect::to(&crate::routes::MachineDefaultRouteTag.url()).into_response();
    }
    let parsed = parse_machine_form(&form);
    let (name, cost_formula, variables) = match parsed {
        Ok(v) => v,
        Err(error) => {
            let page = MachineCreateModalPage {
                form_name: q.form_name(),
                refresh_table: q.refresh_table(),
                target_input: q.target_input(),
                name: form.name,
                cost_formula: form.cost_formula,
                variables: form.variables,
                error,
            };
            return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
                .into_response();
        }
    };
    let product_id = match crate::machine_product::save_machine_product(
        &state.db,
        0,
        &name,
        &variables,
        &cost_formula,
    )
    .await
    {
        Ok(id) => id,
        Err(error) => {
            let page = MachineCreateModalPage {
                form_name: q.form_name(),
                refresh_table: q.refresh_table(),
                target_input: q.target_input(),
                name: form.name,
                cost_formula: form.cost_formula,
                variables: form.variables,
                error,
            };
            return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
                .into_response();
        }
    };
    let now = Utc::now();
    let model = machine::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        name: Set(name),
        product_id: Set(product_id),
        cost_formula: Set(cost_formula),
        variables: Set(variables),
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
                cost_formula: form.cost_formula,
                variables: form.variables,
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
    if !can_manage(&ctx) {
        return Redirect::to(&crate::routes::MachineDefaultRouteTag.url()).into_response();
    }
    let Some(m) = find_machine_scoped(&state.db, id, &ctx).await else {
        return Redirect::to(&crate::routes::MachineDefaultRouteTag.url()).into_response();
    };
    let variables = schema_entries_for(&m);
    let page = MachineEditModalPage {
        id: m.id,
        form_name: q.form_name(),
        name: m.name,
        cost_formula: m.cost_formula,
        variables,
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
    if !can_manage(&ctx) {
        return Redirect::to(&crate::routes::MachineDefaultRouteTag.url()).into_response();
    }
    let Some(existing) = find_machine_scoped(&state.db, id, &ctx).await else {
        return Redirect::to(&crate::routes::MachineDefaultRouteTag.url()).into_response();
    };
    let parsed = parse_machine_form(&form);
    let (name, cost_formula, variables) = match parsed {
        Ok(v) => v,
        Err(error) => {
            let page = MachineEditModalPage {
                id,
                form_name: q.form_name(),
                name: form.name,
                cost_formula: form.cost_formula,
                variables: form.variables,
                error,
            };
            return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
                .into_response();
        }
    };
    let product_id = match crate::machine_product::save_machine_product(
        &state.db,
        existing.product_id,
        &name,
        &variables,
        &cost_formula,
    )
    .await
    {
        Ok(id) => id,
        Err(error) => {
            let page = MachineEditModalPage {
                id,
                form_name: q.form_name(),
                name: form.name,
                cost_formula: form.cost_formula,
                variables: form.variables,
                error,
            };
            return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
                .into_response();
        }
    };
    let now = Utc::now();
    let mut am: machine::ActiveModel = existing.into();
    am.updated_at = Set(Some(now));
    am.name = Set(name);
    am.product_id = Set(product_id);
    am.cost_formula = Set(cost_formula);
    am.variables = Set(variables);
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
                cost_formula: form.cost_formula,
                variables: form.variables,
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
        post_url: crate::routes::MachineDeletePostRouteTag::new(id).url(),
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
    let list_url = crate::routes::MachineDefaultRouteTag.url();
    if !can_manage(&ctx) {
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
                post_url: crate::routes::MachineDeletePostRouteTag::new(id).url(),
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
    let prepared = prepare(
        uri.query().unwrap_or(""),
        machine_fields(),
        state.db.get_database_backend(),
    );
    let page_size = q.filter.page_size.get();
    let machines = load_machine_rows(
        &state.db,
        q.filter.sort.as_deref(),
        q.filter.page.get(),
        page_size,
        &ctx,
        prepared.condition,
    )
    .await;
    let page = MachineSelectPage {
        machines,
        fields: prepared.fields,
        filters: prepared.filters,
        filter_errors: prepared.errors,
        sort: q.filter.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        page_size,
        target_input: q.target_input.clone().unwrap_or_else(|| "Machines".into()),
        can_edit: can_manage(&ctx),
        multi: query_is_multi(q.multi.as_deref()),
    };
    respond_picker_select::<MachineSelectTableKey, MachineSelectModalKey, _>(&htmx, &page)
}
