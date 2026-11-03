//! Work order HTTP handlers.

use std::collections::HashMap;

use axum::extract::{Form, Path, Query};
use axum::http::Uri;
use axum::response::{IntoResponse, Response};
use rust_decimal::Decimal;
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
};
use serde::Deserialize;
use std::str::FromStr;

use lariv_core::components::{ObjectList, SharedChromeFolder, SlotCtx, SwapKey};
use lariv_core::html_form::HtmlFormBody;
use lariv_core::http::Cap;
use lariv_core::template::RenderAppPane;
use lariv_core::web::{
    Htmx, ModalFormQuery, QueryPage, QueryPageSize, html_built_page_or_app_layout,
    html_built_page_with_slots, respond_create_modal_done, respond_edit_modal_done,
};
use lariv_plugin_customer::entities::customer::Entity as CustomerEntity;
use lariv_plugin_customer::logic::{detail_href, display_name, require_existing};
use lariv_plugin_filesystem::entities::filesystem_node::Entity as VNodeEntity;
use lariv_plugin_filesystem::routes::VNodeDetailRouteTag;
use lariv_plugin_users::middleware::RequireAuth;
use lariv_plugin_users::state::AuthContext;

use crate::entities::{
    CompletedStepEntity, PartEntity, PresuppliedPartEntity, StepEntity, StepInputEntity,
    StepOutputEntity, WorkOrderEntity, completed_step, part, step, step_input, step_output,
    work_order,
};
use crate::forms::{FlowForm, PartForm, PresuppliedForm, StepForm, WorkOrderForm};
use crate::keys::*;
use crate::logic::{self, ActualLine, FlowInput, FlowSide, PartInput, PresuppliedInput, StepInput};
use crate::measure;
use crate::plan_lines;
use crate::routes::WorkOrderListRouteTag;
use crate::state::WorkOrdersState;
use crate::templates::{
    CompleteLineView, CompleteModalPage, ConfirmDeleteModalPage, FlowModalPage, PartModalPage,
    PresuppliedModalPage, StepModalPage, WorkOrderCreateModalPage, WorkOrderDetailPage,
    WorkOrderEditModalPage, WorkOrderListPage, WorkOrderRow, detail_redirect,
};

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    #[serde(default)]
    pub page: QueryPage,
    #[serde(default)]
    pub page_size: QueryPageSize,
}

fn path_and_query(uri: &Uri) -> String {
    uri.path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string())
}

fn modal_response(
    page: &impl lariv_core::template::RenderTemplate,
    chrome: &SharedChromeFolder,
    ctx: &AuthContext,
) -> Response {
    html_built_page_with_slots(page, chrome, &SlotCtx::from_auth(ctx)).into_response()
}

fn reload(htmx: &Htmx, modal: SwapKey, work_order_id: i64) -> Response {
    respond_edit_modal_done(htmx, modal, &detail_redirect(work_order_id))
}

fn list_redirect(htmx: &Htmx) -> Response {
    htmx.redirect(&WorkOrderListRouteTag.url())
}

fn part_choices(parts: &[part::Model]) -> Vec<(String, String)> {
    parts
        .iter()
        .map(|part| {
            (
                part.id.to_string(),
                format!("{} ({})", part.name, part.measure_unit),
            )
        })
        .collect()
}

fn parse_part_id(raw: &str) -> Result<i64, String> {
    let id = raw
        .trim()
        .parse::<i64>()
        .map_err(|_| "Choose a part.".to_string())?;
    if id <= 0 {
        Err("Choose a part.".to_string())
    } else {
        Ok(id)
    }
}

fn parse_batches(raw: &str) -> Result<i32, String> {
    let count = raw
        .trim()
        .parse::<i32>()
        .map_err(|_| "Enter a whole number of batches.".to_string())?;
    if count <= 0 {
        Err("Planned batches must be at least 1.".to_string())
    } else {
        Ok(count)
    }
}

pub async fn list(
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<ListQuery>,
) -> maud::Markup {
    let page_num = q.page.get();
    let page_size = q.page_size.get();
    let paginator = WorkOrderEntity::find()
        .order_by_desc(work_order::Column::Id)
        .paginate(&state.db, u64::from(page_size));
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page(u64::from(page_num.saturating_sub(1)))
        .await
        .unwrap_or_default();
    let mut rows = Vec::with_capacity(models.len());
    for order in models {
        let customer = display_name(&state.db, order.customer_id).await;
        let progress = step_progress(&state.db, order.id).await;
        rows.push(WorkOrderRow {
            id: order.id,
            title: order.title,
            customer,
            progress,
        });
    }
    let page = WorkOrderListPage {
        orders: ObjectList::from_page(rows, page_num, page_size, total),
        path_and_query: path_and_query(&uri),
    };
    if htmx.targets(WORK_ORDER_TABLE) {
        return page.render_table();
    }
    if htmx.wants_main_content() {
        return page.render_main().into();
    }
    if htmx.wants_app_layout() {
        return page.render_pane().into();
    }
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

async fn step_progress(db: &sea_orm::DatabaseConnection, work_order_id: i64) -> String {
    let steps = StepEntity::find()
        .filter(step::Column::WorkOrderId.eq(work_order_id))
        .all(db)
        .await
        .unwrap_or_default();
    if steps.is_empty() {
        return "0/0".to_string();
    }
    let ids: Vec<i64> = steps.iter().map(|step| step.id).collect();
    let completions = CompletedStepEntity::find()
        .filter(completed_step::Column::StepId.is_in(ids))
        .all(db)
        .await
        .unwrap_or_default();
    let done = steps
        .iter()
        .filter(|step| {
            let count = completions
                .iter()
                .filter(|row| row.step_id == step.id)
                .count();
            i32::try_from(count).unwrap_or(i32::MAX) >= step.planned_batches
        })
        .count();
    format!("{done}/{}", steps.len())
}

pub async fn create_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalFormQuery>,
) -> maud::Markup {
    let page = WorkOrderCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        title: String::new(),
        customer_id: String::new(),
        customer_display: String::new(),
        folder_vnode_id: String::new(),
        folder_display: String::new(),
        parts_json: "[]".to_string(),
        presupplied_json: "[]".to_string(),
        steps_json: "[]".to_string(),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

pub async fn create_post(
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalFormQuery>,
    HtmlFormBody(form): HtmlFormBody<WorkOrderForm>,
) -> Response {
    let draft = match plan_lines::parse_plan(
        &form.parts_lines_json,
        &form.presupplied_lines_json,
        &form.steps_lines_json,
    ) {
        Ok(draft) => draft,
        Err(error) => return create_error(&state, &chrome, &ctx, &q, &form, error).await,
    };
    let customer_id = match require_existing(&state.db, form.customer_id).await {
        Ok(id) => id,
        Err(error) => return create_error(&state, &chrome, &ctx, &q, &form, error).await,
    };
    let folder = match require_vnode(&state.db, form.folder_vnode_id).await {
        Ok(id) => id,
        Err(error) => return create_error(&state, &chrome, &ctx, &q, &form, error).await,
    };
    let saved = match logic::create_work_order(
        &state.db,
        logic::NewWorkOrder {
            title: form.title.clone(),
            customer_id,
            folder_vnode_id: folder,
        },
    )
    .await
    {
        Ok(saved) => saved,
        Err(error) => return create_error(&state, &chrome, &ctx, &q, &form, error).await,
    };
    if let Err(error) = logic::replace_plan_lines(&state.db, saved.id, &draft).await {
        let _ = logic::delete_work_order(&state.db, saved.id).await;
        return create_error(&state, &chrome, &ctx, &q, &form, error).await;
    }
    respond_create_modal_done(
        &htmx,
        WORK_ORDER_CREATE_MODAL,
        "",
        &detail_redirect(saved.id),
    )
}

async fn create_error(
    state: &WorkOrdersState,
    chrome: &SharedChromeFolder,
    ctx: &AuthContext,
    q: &ModalFormQuery,
    form: &WorkOrderForm,
    error: String,
) -> Response {
    let fields = form_fields(&state.db, form).await;
    let page = WorkOrderCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        title: fields.title,
        customer_id: fields.customer_id,
        customer_display: fields.customer_display,
        folder_vnode_id: fields.folder_vnode_id,
        folder_display: fields.folder_display,
        parts_json: fields.parts_json,
        presupplied_json: fields.presupplied_json,
        steps_json: fields.steps_json,
        error,
    };
    modal_response(&page, chrome, ctx)
}

pub async fn detail(
    Path(id): Path<i64>,
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
) -> Response {
    let Some(page) = detail_page(&state, id).await else {
        return list_redirect(&htmx);
    };
    if htmx.targets(WORK_ORDER_GRAPH) {
        return page.render_graph().into_response();
    }
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

async fn detail_page(state: &WorkOrdersState, id: i64) -> Option<WorkOrderDetailPage> {
    let plan = logic::load_plan(&state.db, id).await.ok()?;
    let customer = display_name(&state.db, plan.order.customer_id).await;
    let customer_href = CustomerEntity::find_by_id(plan.order.customer_id)
        .one(&state.db)
        .await
        .ok()
        .flatten()
        .map(|row| detail_href(&row))
        .unwrap_or_default();
    let files_href = VNodeDetailRouteTag::new(plan.order.folder_vnode_id).url();
    let files_label = vnode_label(&state.db, plan.order.folder_vnode_id).await;
    let (layout, layout_error) = match logic::plan_graph(&plan) {
        Ok(graph) => match crate::graph::layout(&graph) {
            Ok(layout) => (Some(layout), String::new()),
            Err(crate::graph::Cycle) => (None, "This plan has a cycle.".to_string()),
        },
        Err(error) => (None, error),
    };
    Some(WorkOrderDetailPage {
        plan,
        customer,
        customer_href,
        files_href,
        files_label,
        layout,
        layout_error,
    })
}

pub async fn edit_get(
    Path(id): Path<i64>,
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalFormQuery>,
) -> Response {
    let Some(plan) = logic::load_plan(&state.db, id).await.ok() else {
        return list_redirect(&htmx);
    };
    let encoded = plan_lines::encode_plan(
        &plan.parts,
        &plan.presupplied,
        &plan.steps,
        &plan.inputs,
        &plan.outputs,
    );
    let page = WorkOrderEditModalPage {
        id,
        form_name: q.form_name(),
        title: plan.order.title.clone(),
        customer_id: plan.order.customer_id.to_string(),
        customer_display: display_name(&state.db, plan.order.customer_id).await,
        folder_vnode_id: plan.order.folder_vnode_id.to_string(),
        folder_display: vnode_label(&state.db, plan.order.folder_vnode_id).await,
        parts_json: encoded.parts_json,
        presupplied_json: encoded.presupplied_json,
        steps_json: encoded.steps_json,
        error: String::new(),
    };
    modal_response(&page, &chrome, &ctx)
}

pub async fn edit_post(
    Path(id): Path<i64>,
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalFormQuery>,
    HtmlFormBody(form): HtmlFormBody<WorkOrderForm>,
) -> Response {
    let draft = match plan_lines::parse_plan(
        &form.parts_lines_json,
        &form.presupplied_lines_json,
        &form.steps_lines_json,
    ) {
        Ok(draft) => draft,
        Err(error) => return edit_error(&state, &chrome, &ctx, id, &q, &form, error).await,
    };
    let customer_id = match require_existing(&state.db, form.customer_id).await {
        Ok(customer_id) => customer_id,
        Err(error) => return edit_error(&state, &chrome, &ctx, id, &q, &form, error).await,
    };
    let folder = match require_vnode(&state.db, form.folder_vnode_id).await {
        Ok(folder) => folder,
        Err(error) => return edit_error(&state, &chrome, &ctx, id, &q, &form, error).await,
    };
    if let Err(error) =
        logic::update_work_order(&state.db, id, &form.title, customer_id, folder).await
    {
        return edit_error(&state, &chrome, &ctx, id, &q, &form, error).await;
    }
    match logic::replace_plan_lines(&state.db, id, &draft).await {
        Ok(()) => reload(&htmx, WORK_ORDER_EDIT_MODAL, id),
        Err(error) => edit_error(&state, &chrome, &ctx, id, &q, &form, error).await,
    }
}

async fn edit_error(
    state: &WorkOrdersState,
    chrome: &SharedChromeFolder,
    ctx: &AuthContext,
    id: i64,
    q: &ModalFormQuery,
    form: &WorkOrderForm,
    error: String,
) -> Response {
    let fields = form_fields(&state.db, form).await;
    let page = WorkOrderEditModalPage {
        id,
        form_name: q.form_name(),
        title: fields.title,
        customer_id: fields.customer_id,
        customer_display: fields.customer_display,
        folder_vnode_id: fields.folder_vnode_id,
        folder_display: fields.folder_display,
        parts_json: fields.parts_json,
        presupplied_json: fields.presupplied_json,
        steps_json: fields.steps_json,
        error,
    };
    modal_response(&page, chrome, ctx)
}

struct PostedOrderForm {
    title: String,
    customer_id: String,
    customer_display: String,
    folder_vnode_id: String,
    folder_display: String,
    parts_json: String,
    presupplied_json: String,
    steps_json: String,
}

async fn form_fields(db: &DatabaseConnection, form: &WorkOrderForm) -> PostedOrderForm {
    PostedOrderForm {
        title: form.title.clone(),
        customer_id: positive_id(form.customer_id),
        customer_display: if form.customer_id > 0 {
            display_name(db, form.customer_id).await
        } else {
            String::new()
        },
        folder_vnode_id: positive_id(form.folder_vnode_id),
        folder_display: vnode_label(db, form.folder_vnode_id).await,
        parts_json: form.parts_lines_json.clone(),
        presupplied_json: form.presupplied_lines_json.clone(),
        steps_json: form.steps_lines_json.clone(),
    }
}

fn positive_id(id: i64) -> String {
    if id > 0 {
        id.to_string()
    } else {
        String::new()
    }
}

async fn require_vnode(db: &DatabaseConnection, id: i64) -> Result<i64, String> {
    if id <= 0 {
        return Err("Select a file or folder.".to_string());
    }
    let found = VNodeEntity::find_by_id(id)
        .one(db)
        .await
        .map_err(|err| err.to_string())?;
    if found.is_none() {
        return Err("Select a file or folder.".to_string());
    }
    Ok(id)
}

async fn vnode_label(db: &DatabaseConnection, id: i64) -> String {
    if id <= 0 {
        return "Files".to_string();
    }
    VNodeEntity::find_by_id(id)
        .one(db)
        .await
        .ok()
        .flatten()
        .map(|node| node.name)
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "Files".to_string())
}

pub async fn delete_get(
    Path(id): Path<i64>,
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
) -> Response {
    if WorkOrderEntity::find_by_id(id)
        .one(&state.db)
        .await
        .ok()
        .flatten()
        .is_none()
    {
        return list_redirect(&htmx);
    }
    modal_response(
        &confirm(
            WORK_ORDER_DELETE_MODAL.id(),
            "Delete work order",
            "This deletes the plan. Recorded batches must be removed first.",
            &crate::routes::WorkOrderDeletePostRouteTag::new(id).url(),
            "",
        ),
        &chrome,
        &ctx,
    )
}

pub async fn delete_post(
    Path(id): Path<i64>,
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
) -> Response {
    match logic::delete_work_order(&state.db, id).await {
        Ok(()) => list_redirect(&htmx),
        Err(error) => modal_response(
            &confirm(
                WORK_ORDER_DELETE_MODAL.id(),
                "Delete work order",
                "This deletes the plan. Recorded batches must be removed first.",
                &crate::routes::WorkOrderDeletePostRouteTag::new(id).url(),
                &error,
            ),
            &chrome,
            &ctx,
        ),
    }
}

pub async fn part_create_get(
    Path(id): Path<i64>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalFormQuery>,
) -> maud::Markup {
    let page = PartModalPage {
        work_order_id: id,
        existing_id: None,
        form_name: q.form_name(),
        name: String::new(),
        measure_kind: "quantity".to_string(),
        measure_unit: "Number".to_string(),
        kind_choices: measure::kind_choices(),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

pub async fn part_create_post(
    Path(id): Path<i64>,
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalFormQuery>,
    HtmlFormBody(form): HtmlFormBody<PartForm>,
) -> Response {
    match logic::save_part(&state.db, part_input(id, None, &form)).await {
        Ok(saved) => reload(&htmx, PART_CREATE_MODAL, saved.work_order_id),
        Err(error) => modal_response(
            &PartModalPage {
                work_order_id: id,
                existing_id: None,
                form_name: q.form_name(),
                name: form.name,
                measure_kind: form.measure_kind,
                measure_unit: form.measure_unit,
                kind_choices: measure::kind_choices(),
                error,
            },
            &chrome,
            &ctx,
        ),
    }
}

pub async fn part_edit_get(
    Path(id): Path<i64>,
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalFormQuery>,
) -> Response {
    let Some(part) = PartEntity::find_by_id(id)
        .one(&state.db)
        .await
        .ok()
        .flatten()
    else {
        return list_redirect(&htmx);
    };
    modal_response(
        &PartModalPage {
            work_order_id: part.work_order_id,
            existing_id: Some(part.id),
            form_name: q.form_name(),
            name: part.name,
            measure_kind: part.measure_kind,
            measure_unit: part.measure_unit,
            kind_choices: measure::kind_choices(),
            error: String::new(),
        },
        &chrome,
        &ctx,
    )
}

pub async fn part_edit_post(
    Path(id): Path<i64>,
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalFormQuery>,
    HtmlFormBody(form): HtmlFormBody<PartForm>,
) -> Response {
    let Some(existing) = PartEntity::find_by_id(id)
        .one(&state.db)
        .await
        .ok()
        .flatten()
    else {
        return list_redirect(&htmx);
    };
    match logic::save_part(
        &state.db,
        part_input(existing.work_order_id, Some(id), &form),
    )
    .await
    {
        Ok(saved) => reload(&htmx, PART_EDIT_MODAL, saved.work_order_id),
        Err(error) => modal_response(
            &PartModalPage {
                work_order_id: existing.work_order_id,
                existing_id: Some(id),
                form_name: q.form_name(),
                name: form.name,
                measure_kind: form.measure_kind,
                measure_unit: form.measure_unit,
                kind_choices: measure::kind_choices(),
                error,
            },
            &chrome,
            &ctx,
        ),
    }
}

pub async fn part_delete_get(
    Path(id): Path<i64>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
) -> maud::Markup {
    let page = confirm(
        PART_DELETE_MODAL.id(),
        "Delete part",
        "Remove this part from the work order.",
        &crate::routes::PartDeletePostRouteTag::new(id).url(),
        "",
    );
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

pub async fn part_delete_post(
    Path(id): Path<i64>,
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
) -> Response {
    match logic::delete_part(&state.db, id).await {
        Ok(work_order_id) => reload(&htmx, PART_DELETE_MODAL, work_order_id),
        Err(error) => modal_response(
            &confirm(
                PART_DELETE_MODAL.id(),
                "Delete part",
                "Remove this part from the work order.",
                &crate::routes::PartDeletePostRouteTag::new(id).url(),
                &error,
            ),
            &chrome,
            &ctx,
        ),
    }
}

pub async fn presupplied_create_get(
    Path(id): Path<i64>,
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalFormQuery>,
) -> maud::Markup {
    let parts = parts_for(&state, id).await;
    let page = PresuppliedModalPage {
        work_order_id: id,
        existing_id: None,
        form_name: q.form_name(),
        part_id: String::new(),
        qty_value: String::new(),
        qty_unit: String::new(),
        part_choices: part_choices(&parts),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

pub async fn presupplied_create_post(
    Path(id): Path<i64>,
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalFormQuery>,
    HtmlFormBody(form): HtmlFormBody<PresuppliedForm>,
) -> Response {
    let parts = parts_for(&state, id).await;
    let part_id = match parse_part_id(&form.part_id) {
        Ok(part_id) => part_id,
        Err(error) => {
            return modal_response(
                &presupplied_page(id, None, &q, &form, &parts, error),
                &chrome,
                &ctx,
            );
        }
    };
    match logic::save_presupplied(
        &state.db,
        PresuppliedInput {
            work_order_id: id,
            existing_id: None,
            part_id,
            qty_value: form.qty_value.clone(),
            qty_unit: form.qty_unit.clone(),
        },
    )
    .await
    {
        Ok(saved) => reload(&htmx, PRESUPPLIED_CREATE_MODAL, saved.work_order_id),
        Err(error) => modal_response(
            &presupplied_page(id, None, &q, &form, &parts, error),
            &chrome,
            &ctx,
        ),
    }
}

pub async fn presupplied_edit_get(
    Path(id): Path<i64>,
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalFormQuery>,
) -> Response {
    let Some(line) = PresuppliedPartEntity::find_by_id(id)
        .one(&state.db)
        .await
        .ok()
        .flatten()
    else {
        return list_redirect(&htmx);
    };
    let parts = parts_for(&state, line.work_order_id).await;
    modal_response(
        &PresuppliedModalPage {
            work_order_id: line.work_order_id,
            existing_id: Some(line.id),
            form_name: q.form_name(),
            part_id: line.part_id.to_string(),
            qty_value: line.qty_value.normalize().to_string(),
            qty_unit: line.qty_unit,
            part_choices: part_choices(&parts),
            error: String::new(),
        },
        &chrome,
        &ctx,
    )
}

pub async fn presupplied_edit_post(
    Path(id): Path<i64>,
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalFormQuery>,
    HtmlFormBody(form): HtmlFormBody<PresuppliedForm>,
) -> Response {
    let Some(existing) = PresuppliedPartEntity::find_by_id(id)
        .one(&state.db)
        .await
        .ok()
        .flatten()
    else {
        return list_redirect(&htmx);
    };
    let parts = parts_for(&state, existing.work_order_id).await;
    let part_id = match parse_part_id(&form.part_id) {
        Ok(part_id) => part_id,
        Err(error) => {
            return modal_response(
                &presupplied_page(existing.work_order_id, Some(id), &q, &form, &parts, error),
                &chrome,
                &ctx,
            );
        }
    };
    match logic::save_presupplied(
        &state.db,
        PresuppliedInput {
            work_order_id: existing.work_order_id,
            existing_id: Some(id),
            part_id,
            qty_value: form.qty_value.clone(),
            qty_unit: form.qty_unit.clone(),
        },
    )
    .await
    {
        Ok(saved) => reload(&htmx, PRESUPPLIED_EDIT_MODAL, saved.work_order_id),
        Err(error) => modal_response(
            &presupplied_page(existing.work_order_id, Some(id), &q, &form, &parts, error),
            &chrome,
            &ctx,
        ),
    }
}

pub async fn presupplied_delete_get(
    Path(id): Path<i64>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
) -> maud::Markup {
    let page = confirm(
        PRESUPPLIED_DELETE_MODAL.id(),
        "Delete supply",
        "Remove this presupplied quantity. Recorded batches stay as they are.",
        &crate::routes::PresuppliedDeletePostRouteTag::new(id).url(),
        "",
    );
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

pub async fn presupplied_delete_post(
    Path(id): Path<i64>,
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
) -> Response {
    match logic::delete_presupplied(&state.db, id).await {
        Ok(work_order_id) => reload(&htmx, PRESUPPLIED_DELETE_MODAL, work_order_id),
        Err(error) => modal_response(
            &confirm(
                PRESUPPLIED_DELETE_MODAL.id(),
                "Delete supply",
                "Remove this presupplied quantity. Recorded batches stay as they are.",
                &crate::routes::PresuppliedDeletePostRouteTag::new(id).url(),
                &error,
            ),
            &chrome,
            &ctx,
        ),
    }
}

pub async fn step_create_get(
    Path(id): Path<i64>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalFormQuery>,
) -> maud::Markup {
    let page = StepModalPage {
        work_order_id: id,
        existing_id: None,
        form_name: q.form_name(),
        name: String::new(),
        planned_batches: "1".to_string(),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

pub async fn step_create_post(
    Path(id): Path<i64>,
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalFormQuery>,
    HtmlFormBody(form): HtmlFormBody<StepForm>,
) -> Response {
    let planned_batches = match parse_batches(&form.planned_batches) {
        Ok(count) => count,
        Err(error) => {
            return modal_response(&step_page(id, None, &q, &form, error), &chrome, &ctx);
        }
    };
    match logic::save_step(
        &state.db,
        StepInput {
            work_order_id: id,
            existing_id: None,
            name: form.name.clone(),
            planned_batches,
        },
    )
    .await
    {
        Ok(saved) => reload(&htmx, STEP_CREATE_MODAL, saved.work_order_id),
        Err(error) => modal_response(&step_page(id, None, &q, &form, error), &chrome, &ctx),
    }
}

pub async fn step_edit_get(
    Path(id): Path<i64>,
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalFormQuery>,
) -> Response {
    let Some(step) = StepEntity::find_by_id(id)
        .one(&state.db)
        .await
        .ok()
        .flatten()
    else {
        return list_redirect(&htmx);
    };
    modal_response(
        &StepModalPage {
            work_order_id: step.work_order_id,
            existing_id: Some(step.id),
            form_name: q.form_name(),
            name: step.name,
            planned_batches: step.planned_batches.to_string(),
            error: String::new(),
        },
        &chrome,
        &ctx,
    )
}

pub async fn step_edit_post(
    Path(id): Path<i64>,
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalFormQuery>,
    HtmlFormBody(form): HtmlFormBody<StepForm>,
) -> Response {
    let Some(existing) = StepEntity::find_by_id(id)
        .one(&state.db)
        .await
        .ok()
        .flatten()
    else {
        return list_redirect(&htmx);
    };
    let planned_batches = match parse_batches(&form.planned_batches) {
        Ok(count) => count,
        Err(error) => {
            return modal_response(
                &step_page(existing.work_order_id, Some(id), &q, &form, error),
                &chrome,
                &ctx,
            );
        }
    };
    match logic::save_step(
        &state.db,
        StepInput {
            work_order_id: existing.work_order_id,
            existing_id: Some(id),
            name: form.name.clone(),
            planned_batches,
        },
    )
    .await
    {
        Ok(saved) => reload(&htmx, STEP_EDIT_MODAL, saved.work_order_id),
        Err(error) => modal_response(
            &step_page(existing.work_order_id, Some(id), &q, &form, error),
            &chrome,
            &ctx,
        ),
    }
}

pub async fn step_delete_get(
    Path(id): Path<i64>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
) -> maud::Markup {
    let page = confirm(
        STEP_DELETE_MODAL.id(),
        "Delete step",
        "Delete recorded batches before deleting this step.",
        &crate::routes::StepDeletePostRouteTag::new(id).url(),
        "",
    );
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

pub async fn step_delete_post(
    Path(id): Path<i64>,
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
) -> Response {
    match logic::delete_step(&state.db, id).await {
        Ok(work_order_id) => reload(&htmx, STEP_DELETE_MODAL, work_order_id),
        Err(error) => modal_response(
            &confirm(
                STEP_DELETE_MODAL.id(),
                "Delete step",
                "Delete recorded batches before deleting this step.",
                &crate::routes::StepDeletePostRouteTag::new(id).url(),
                &error,
            ),
            &chrome,
            &ctx,
        ),
    }
}

pub async fn input_create_get(
    Path(id): Path<i64>,
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalFormQuery>,
) -> Response {
    flow_create_page(&state, &chrome, &ctx, &htmx, id, FlowSide::Input, &q).await
}

pub async fn output_create_get(
    Path(id): Path<i64>,
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalFormQuery>,
) -> Response {
    flow_create_page(&state, &chrome, &ctx, &htmx, id, FlowSide::Output, &q).await
}

pub async fn input_create_post(
    Path(id): Path<i64>,
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalFormQuery>,
    HtmlFormBody(form): HtmlFormBody<FlowForm>,
) -> Response {
    flow_save(
        &modal_ctx(&state, &chrome, &ctx, &htmx),
        id,
        None,
        FlowSide::Input,
        &q,
        form,
    )
    .await
}

pub async fn output_create_post(
    Path(id): Path<i64>,
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalFormQuery>,
    HtmlFormBody(form): HtmlFormBody<FlowForm>,
) -> Response {
    flow_save(
        &modal_ctx(&state, &chrome, &ctx, &htmx),
        id,
        None,
        FlowSide::Output,
        &q,
        form,
    )
    .await
}

pub async fn input_edit_get(
    Path(id): Path<i64>,
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalFormQuery>,
) -> Response {
    flow_edit_page(&state, &chrome, &ctx, &htmx, id, FlowSide::Input, &q).await
}

pub async fn output_edit_get(
    Path(id): Path<i64>,
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalFormQuery>,
) -> Response {
    flow_edit_page(&state, &chrome, &ctx, &htmx, id, FlowSide::Output, &q).await
}

pub async fn input_edit_post(
    Path(id): Path<i64>,
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalFormQuery>,
    HtmlFormBody(form): HtmlFormBody<FlowForm>,
) -> Response {
    flow_edit_save(
        &modal_ctx(&state, &chrome, &ctx, &htmx),
        id,
        FlowSide::Input,
        &q,
        form,
    )
    .await
}

pub async fn output_edit_post(
    Path(id): Path<i64>,
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalFormQuery>,
    HtmlFormBody(form): HtmlFormBody<FlowForm>,
) -> Response {
    flow_edit_save(
        &modal_ctx(&state, &chrome, &ctx, &htmx),
        id,
        FlowSide::Output,
        &q,
        form,
    )
    .await
}

pub async fn input_delete_get(
    Path(id): Path<i64>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
) -> maud::Markup {
    delete_flow_page(&chrome, &ctx, id, FlowSide::Input, "").await
}

pub async fn output_delete_get(
    Path(id): Path<i64>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
) -> maud::Markup {
    delete_flow_page(&chrome, &ctx, id, FlowSide::Output, "").await
}

pub async fn input_delete_post(
    Path(id): Path<i64>,
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
) -> Response {
    delete_flow(&state, &chrome, &ctx, &htmx, id, FlowSide::Input).await
}

pub async fn output_delete_post(
    Path(id): Path<i64>,
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
) -> Response {
    delete_flow(&state, &chrome, &ctx, &htmx, id, FlowSide::Output).await
}

pub async fn complete_get(
    Path(id): Path<i64>,
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalFormQuery>,
) -> Response {
    let Some(step) = StepEntity::find_by_id(id)
        .one(&state.db)
        .await
        .ok()
        .flatten()
    else {
        return list_redirect(&htmx);
    };
    let parts = parts_for(&state, step.work_order_id).await;
    let inputs = StepInputEntity::find()
        .filter(step_input::Column::StepId.eq(id))
        .all(&state.db)
        .await
        .unwrap_or_default();
    let outputs = StepOutputEntity::find()
        .filter(step_output::Column::StepId.eq(id))
        .all(&state.db)
        .await
        .unwrap_or_default();
    let page = CompleteModalPage {
        step_id: id,
        step_name: step.name,
        form_name: q.form_name(),
        inputs: inputs
            .iter()
            .map(|line| complete_line(&parts, line.part_id, &line.qty_value, &line.qty_unit))
            .collect(),
        outputs: outputs
            .iter()
            .map(|line| complete_line(&parts, line.part_id, &line.qty_value, &line.qty_unit))
            .collect(),
        error: String::new(),
    };
    modal_response(&page, &chrome, &ctx)
}

pub async fn complete_post(
    Path(id): Path<i64>,
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalFormQuery>,
    Form(form): Form<HashMap<String, String>>,
) -> Response {
    let Some(step) = StepEntity::find_by_id(id)
        .one(&state.db)
        .await
        .ok()
        .flatten()
    else {
        return list_redirect(&htmx);
    };
    let inputs = match actuals(&form, "in") {
        Ok(lines) => lines,
        Err(error) => return complete_error(&state, &chrome, &ctx, &step, &q, &form, error).await,
    };
    let outputs = match actuals(&form, "out") {
        Ok(lines) => lines,
        Err(error) => return complete_error(&state, &chrome, &ctx, &step, &q, &form, error).await,
    };
    match logic::complete_step(&state.db, id, ctx.user.id, inputs, outputs).await {
        Ok(_) => reload(&htmx, COMPLETE_MODAL, step.work_order_id),
        Err(error) => complete_error(&state, &chrome, &ctx, &step, &q, &form, error).await,
    }
}

pub async fn completion_delete_get(
    Path(id): Path<i64>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
) -> maud::Markup {
    let page = confirm(
        COMPLETION_DELETE_MODAL.id(),
        "Delete batch",
        "Remove this recorded batch.",
        &crate::routes::CompletionDeletePostRouteTag::new(id).url(),
        "",
    );
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

pub async fn completion_delete_post(
    Path(id): Path<i64>,
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
) -> Response {
    match logic::delete_completion(&state.db, id).await {
        Ok(work_order_id) => reload(&htmx, COMPLETION_DELETE_MODAL, work_order_id),
        Err(error) => modal_response(
            &confirm(
                COMPLETION_DELETE_MODAL.id(),
                "Delete batch",
                "Remove this recorded batch.",
                &crate::routes::CompletionDeletePostRouteTag::new(id).url(),
                &error,
            ),
            &chrome,
            &ctx,
        ),
    }
}

fn confirm(
    modal_uid: &'static str,
    title: &str,
    message: &str,
    post_url: &str,
    error: &str,
) -> ConfirmDeleteModalPage {
    ConfirmDeleteModalPage {
        modal_uid,
        title: title.to_string(),
        message: message.to_string(),
        post_url: post_url.to_string(),
        error: error.to_string(),
    }
}

fn part_input(work_order_id: i64, existing_id: Option<i64>, form: &PartForm) -> PartInput {
    PartInput {
        work_order_id,
        existing_id,
        name: form.name.clone(),
        measure_kind: form.measure_kind.clone(),
        measure_unit: form.measure_unit.clone(),
    }
}

fn presupplied_page(
    work_order_id: i64,
    existing_id: Option<i64>,
    q: &ModalFormQuery,
    form: &PresuppliedForm,
    parts: &[part::Model],
    error: String,
) -> PresuppliedModalPage {
    PresuppliedModalPage {
        work_order_id,
        existing_id,
        form_name: q.form_name(),
        part_id: form.part_id.clone(),
        qty_value: form.qty_value.clone(),
        qty_unit: form.qty_unit.clone(),
        part_choices: part_choices(parts),
        error,
    }
}

fn step_page(
    work_order_id: i64,
    existing_id: Option<i64>,
    q: &ModalFormQuery,
    form: &StepForm,
    error: String,
) -> StepModalPage {
    StepModalPage {
        work_order_id,
        existing_id,
        form_name: q.form_name(),
        name: form.name.clone(),
        planned_batches: form.planned_batches.clone(),
        error,
    }
}

async fn parts_for(state: &WorkOrdersState, work_order_id: i64) -> Vec<part::Model> {
    PartEntity::find()
        .filter(part::Column::WorkOrderId.eq(work_order_id))
        .all(&state.db)
        .await
        .unwrap_or_default()
}

async fn flow_create_page(
    state: &WorkOrdersState,
    chrome: &SharedChromeFolder,
    ctx: &AuthContext,
    htmx: &Htmx,
    step_id: i64,
    side: FlowSide,
    q: &ModalFormQuery,
) -> Response {
    let Some(step) = StepEntity::find_by_id(step_id)
        .one(&state.db)
        .await
        .ok()
        .flatten()
    else {
        return list_redirect(htmx);
    };
    let parts = parts_for(state, step.work_order_id).await;
    modal_response(
        &FlowModalPage {
            step_id,
            existing_id: None,
            side,
            form_name: q.form_name(),
            part_id: String::new(),
            qty_value: String::new(),
            qty_unit: String::new(),
            part_choices: part_choices(&parts),
            error: String::new(),
        },
        chrome,
        ctx,
    )
}

struct ModalCtx<'a> {
    state: &'a WorkOrdersState,
    chrome: &'a SharedChromeFolder,
    ctx: &'a AuthContext,
    htmx: &'a Htmx,
}

fn modal_ctx<'a>(
    state: &'a WorkOrdersState,
    chrome: &'a SharedChromeFolder,
    ctx: &'a AuthContext,
    htmx: &'a Htmx,
) -> ModalCtx<'a> {
    ModalCtx {
        state,
        chrome,
        ctx,
        htmx,
    }
}

async fn flow_save(
    req: &ModalCtx<'_>,
    step_id: i64,
    existing_id: Option<i64>,
    side: FlowSide,
    q: &ModalFormQuery,
    form: FlowForm,
) -> Response {
    let parts = StepEntity::find_by_id(step_id)
        .one(&req.state.db)
        .await
        .ok()
        .flatten();
    let Some(step) = parts else {
        return list_redirect(req.htmx);
    };
    let part_rows = parts_for(req.state, step.work_order_id).await;
    let part_id = match parse_part_id(&form.part_id) {
        Ok(part_id) => part_id,
        Err(error) => {
            return modal_response(
                &flow_page(step_id, existing_id, side, q, &form, &part_rows, error),
                req.chrome,
                req.ctx,
            );
        }
    };
    match logic::save_flow(
        &req.state.db,
        FlowInput {
            step_id,
            existing_id,
            part_id,
            qty_value: form.qty_value.clone(),
            qty_unit: form.qty_unit.clone(),
            side,
        },
    )
    .await
    {
        Ok(work_order_id) => match existing_id {
            Some(_) => reload(req.htmx, FLOW_EDIT_MODAL, work_order_id),
            None => reload(req.htmx, FLOW_CREATE_MODAL, work_order_id),
        },
        Err(error) => modal_response(
            &flow_page(step_id, existing_id, side, q, &form, &part_rows, error),
            req.chrome,
            req.ctx,
        ),
    }
}

async fn flow_edit_page(
    state: &WorkOrdersState,
    chrome: &SharedChromeFolder,
    ctx: &AuthContext,
    htmx: &Htmx,
    id: i64,
    side: FlowSide,
    q: &ModalFormQuery,
) -> Response {
    let line = match side {
        FlowSide::Input => StepInputEntity::find_by_id(id)
            .one(&state.db)
            .await
            .ok()
            .flatten()
            .map(|line| (line.step_id, line.part_id, line.qty_value, line.qty_unit)),
        FlowSide::Output => StepOutputEntity::find_by_id(id)
            .one(&state.db)
            .await
            .ok()
            .flatten()
            .map(|line| (line.step_id, line.part_id, line.qty_value, line.qty_unit)),
    };
    let Some((step_id, part_id, qty_value, qty_unit)) = line else {
        return list_redirect(htmx);
    };
    let Some(step) = StepEntity::find_by_id(step_id)
        .one(&state.db)
        .await
        .ok()
        .flatten()
    else {
        return list_redirect(htmx);
    };
    let parts = parts_for(state, step.work_order_id).await;
    modal_response(
        &FlowModalPage {
            step_id,
            existing_id: Some(id),
            side,
            form_name: q.form_name(),
            part_id: part_id.to_string(),
            qty_value: qty_value.normalize().to_string(),
            qty_unit,
            part_choices: part_choices(&parts),
            error: String::new(),
        },
        chrome,
        ctx,
    )
}

async fn flow_edit_save(
    req: &ModalCtx<'_>,
    id: i64,
    side: FlowSide,
    q: &ModalFormQuery,
    form: FlowForm,
) -> Response {
    let step_id = match side {
        FlowSide::Input => StepInputEntity::find_by_id(id)
            .one(&req.state.db)
            .await
            .ok()
            .flatten()
            .map(|line| line.step_id),
        FlowSide::Output => StepOutputEntity::find_by_id(id)
            .one(&req.state.db)
            .await
            .ok()
            .flatten()
            .map(|line| line.step_id),
    };
    let Some(step_id) = step_id else {
        return list_redirect(req.htmx);
    };
    flow_save(req, step_id, Some(id), side, q, form).await
}

fn flow_page(
    step_id: i64,
    existing_id: Option<i64>,
    side: FlowSide,
    q: &ModalFormQuery,
    form: &FlowForm,
    parts: &[part::Model],
    error: String,
) -> FlowModalPage {
    FlowModalPage {
        step_id,
        existing_id,
        side,
        form_name: q.form_name(),
        part_id: form.part_id.clone(),
        qty_value: form.qty_value.clone(),
        qty_unit: form.qty_unit.clone(),
        part_choices: part_choices(parts),
        error,
    }
}

async fn delete_flow_page(
    chrome: &SharedChromeFolder,
    ctx: &AuthContext,
    id: i64,
    side: FlowSide,
    error: &str,
) -> maud::Markup {
    let url = match side {
        FlowSide::Input => crate::routes::InputDeletePostRouteTag::new(id).url(),
        FlowSide::Output => crate::routes::OutputDeletePostRouteTag::new(id).url(),
    };
    let page = confirm(
        FLOW_DELETE_MODAL.id(),
        "Delete line",
        "Remove this quantity from the step.",
        &url,
        error,
    );
    html_built_page_with_slots(&page, chrome, &SlotCtx::from_auth(ctx))
}

async fn delete_flow(
    state: &WorkOrdersState,
    chrome: &SharedChromeFolder,
    ctx: &AuthContext,
    htmx: &Htmx,
    id: i64,
    side: FlowSide,
) -> Response {
    match logic::delete_flow(&state.db, id, side).await {
        Ok(work_order_id) => reload(htmx, FLOW_DELETE_MODAL, work_order_id),
        Err(error) => delete_flow_page(chrome, ctx, id, side, &error)
            .await
            .into_response(),
    }
}

fn complete_line(
    parts: &[part::Model],
    part_id: i64,
    qty_value: &Decimal,
    qty_unit: &str,
) -> CompleteLineView {
    let name = parts
        .iter()
        .find(|part| part.id == part_id)
        .map(|part| part.name.clone())
        .unwrap_or_else(|| "Part".to_string());
    CompleteLineView {
        part_id,
        name,
        qty_value: qty_value.normalize().to_string(),
        qty_unit: qty_unit.to_string(),
    }
}

fn actuals(form: &HashMap<String, String>, prefix: &str) -> Result<Vec<ActualLine>, String> {
    let value_prefix = format!("{prefix}_value_");
    let mut lines = Vec::new();
    for (key, value) in form {
        let Some(id_raw) = key.strip_prefix(&value_prefix) else {
            continue;
        };
        let part_id = parse_part_id(id_raw)?;
        let unit = form
            .get(&format!("{prefix}_unit_{part_id}"))
            .cloned()
            .unwrap_or_default();
        let qty_value =
            Decimal::from_str(value.trim()).map_err(|_| "Enter a quantity.".to_string())?;
        lines.push(ActualLine {
            part_id,
            qty_value,
            qty_unit: unit,
        });
    }
    Ok(lines)
}

async fn complete_error(
    state: &WorkOrdersState,
    chrome: &SharedChromeFolder,
    ctx: &AuthContext,
    step: &step::Model,
    q: &ModalFormQuery,
    form: &HashMap<String, String>,
    error: String,
) -> Response {
    let parts = parts_for(state, step.work_order_id).await;
    let inputs = form_lines(form, &parts, "in");
    let outputs = form_lines(form, &parts, "out");
    modal_response(
        &CompleteModalPage {
            step_id: step.id,
            step_name: step.name.clone(),
            form_name: q.form_name(),
            inputs,
            outputs,
            error,
        },
        chrome,
        ctx,
    )
}

fn form_lines(
    form: &HashMap<String, String>,
    parts: &[part::Model],
    prefix: &str,
) -> Vec<CompleteLineView> {
    let value_prefix = format!("{prefix}_value_");
    let mut lines = Vec::new();
    for (key, value) in form {
        let Some(id_raw) = key.strip_prefix(&value_prefix) else {
            continue;
        };
        let Ok(part_id) = parse_part_id(id_raw) else {
            continue;
        };
        let unit = form
            .get(&format!("{prefix}_unit_{part_id}"))
            .cloned()
            .unwrap_or_default();
        let name = parts
            .iter()
            .find(|part| part.id == part_id)
            .map(|part| part.name.clone())
            .unwrap_or_else(|| "Part".to_string());
        lines.push(CompleteLineView {
            part_id,
            name,
            qty_value: value.clone(),
            qty_unit: unit,
        });
    }
    lines
}
