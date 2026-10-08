//! Machines inside KDS Quotations.
//!
//! The records are the machinery-schedule machines. These pages show the same
//! list and detail content, with the quotations sidebar.

use axum::{
    extract::{Path, Query},
    http::Uri,
    response::{IntoResponse, Redirect, Response},
};
use chrono::Utc;
use frunk::Generic;
use lariv_core::{
    components::{
        ButtonModalForm, ButtonSubmit, DEFAULT_PAGE_SIZE, DetailHeader, FieldDuration, FieldText,
        FormOpts, LayoutMain, LayoutSidebar, ObjectList, PaginationPage, ShellChrome,
        ShellScaffold, SwapKey, TableButtonFilter, TableColumnHeader, TablePagination, TableRow,
        button_modal_form, button_submit, column_sort_url, container_column,
        data_table_list_refresh, detail as detail_panel, detail_header, field_duration, field_text,
        form, form_hx_get_route, form_hx_post_url, label, layout_main, layout_sidebar, modal_keyed,
        pagination_pages, row_attr_navigate, row_attr_navigate_route, shell_scaffold,
        sort_indicator, table_button_filter, table_create_button, table_pagination,
    },
    html_form::{CsrfToken, FormCtx, HtmlForm},
    http::Cap,
    template::{RenderAppPane, RenderTemplate},
    web::{
        Htmx, ModalFormQuery, QueryPage, html_built_page_or_app_layout, html_built_page_with_slots,
        modal_create_post_query, modal_edit_post_url, respond_create_modal_done_fk,
        respond_edit_modal_done,
    },
};
use lariv_plugin_users::middleware::RequireAuth;
use maud::{Markup, html};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait, PaginatorTrait};

use lariv_formula::{parse_schema, parse_schema_list, schema_to_entries, schema_to_json};
use kds_plugin_machinery_schedule::entities::machine::{self, Entity as MachineEntity};
use kds_plugin_machinery_schedule::forms::{
    MachineFilterForm, MachineFilterFormField, MachineForm, MachineFormField,
};
use kds_plugin_machinery_schedule::logic::{
    completed_job_id_for_job, format_job_duration, jobs_for_machine, machine_free_on,
    machine_remaining_duration,
};
use kds_plugin_machinery_schedule::routes::{CompletedJobDetailRouteTag, JobDetailRouteTag};
use kds_plugin_machinery_schedule::scope::{
    apply_name_filter, apply_name_sort_or_id_desc, can_manage, find_machine_scoped,
    scope_superuser, sort_jobs_by_column,
};

use super::crumbs::{machine_crumbs, machines_list_crumbs, wo_menu};
use super::keys::{
    WorkOrdersMachineCreateModalKey, WorkOrdersMachineDeleteModalKey,
    WorkOrdersMachineEditModalKey, WorkOrdersMachineJobsTableKey, WorkOrdersMachineTableKey,
};
use super::routes::{
    WorkOrdersMachineCreatePostRouteTag, WorkOrdersMachineDeleteGetRouteTag,
    WorkOrdersMachineDeletePostRouteTag, WorkOrdersMachineDetailRouteTag,
    WorkOrdersMachineEditGetRouteTag, WorkOrdersMachineEditPostRouteTag,
    WorkOrdersMachinesRouteTag,
};
use super::state::WorkOrdersState;
use super::templates::ConfirmDeleteModalPage;

#[derive(Debug, serde::Deserialize, Default)]
pub struct MachineListQuery {
    #[serde(default, rename = "Name", alias = "name")]
    name: Option<String>,
    #[serde(default)]
    sort: Option<String>,
    #[serde(default)]
    page: QueryPage,
}

#[derive(Debug, serde::Deserialize, Default)]
pub struct MachineDetailQuery {
    #[serde(default)]
    sort: Option<String>,
}

fn path_and_query(uri: &Uri) -> String {
    uri.path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string())
}

fn machine_detail_sort(sort: Option<&str>) -> String {
    sort.map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("Order")
        .to_string()
}

fn list_url() -> String {
    WorkOrdersMachinesRouteTag.url()
}

#[derive(Clone)]
pub struct MachineRow {
    id: i64,
    name: String,
    formula_label: String,
}

fn machine_row(m: &machine::Model) -> MachineRow {
    MachineRow {
        id: m.id,
        name: m.name.clone(),
        formula_label: m.formula_label(),
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
        cost_formula: cost_formula.clone(),
        variables: variables.clone(),
    };
    temp.validate_formulas().map_err(|e| e.to_string())?;
    Ok((name, cost_formula, variables))
}

async fn load_machine_rows(
    db: &sea_orm::DatabaseConnection,
    q: &MachineListQuery,
    auth: &lariv_plugin_users::state::AuthContext,
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
    let rows = models.into_iter().map(|m| machine_row(&m)).collect();
    ObjectList::from_page(rows, page, page_size, total)
}

fn app_scaffold(
    title: &str,
    chrome: &ShellChrome,
    sidebar: Markup,
    crumbs: Markup,
    body: Markup,
) -> Markup {
    shell_scaffold(ShellScaffold {
        title,
        registry_head: chrome.head.clone(),
        topbar_items: chrome.topbar_items.clone(),
        right_sidebar: chrome.right_sidebar.clone(),
        sidebar,
        breadcrumbs: crumbs,
        body,
        ..Default::default()
    })
}

fn scaffold_pane(
    sidebar: Markup,
    crumbs: Markup,
    body: Markup,
) -> lariv_core::components::AppLayoutHtml {
    layout_sidebar(LayoutSidebar {
        sidebar,
        breadcrumbs: crumbs,
        content: body,
    })
}

fn scaffold_main(crumbs: Markup, body: Markup) -> lariv_core::components::MainContentHtml {
    layout_main(LayoutMain {
        breadcrumbs: crumbs,
        content: body,
    })
}

fn render_pagination<K: SwapKey>(path_and_query: &str, number: u32, num_pages: u32) -> Markup {
    let owned = pagination_pages(path_and_query, number, num_pages, true);
    let pages: Vec<PaginationPage<'_>> = owned
        .iter()
        .map(|(ellipsis, url, push_url, active, label)| PaginationPage {
            ellipsis: *ellipsis,
            url: url.as_str(),
            push_url: *push_url,
            active: *active,
            label: label.as_str(),
        })
        .collect();
    table_pagination(TablePagination {
        pages: &pages,
        hx_target: K::SELECTOR,
    })
}

#[derive(Generic)]
pub struct WorkOrdersMachineListPage {
    pub machines: ObjectList<MachineRow>,
    pub filter_name: String,
    pub sort: String,
    pub path_and_query: String,
    pub can_edit: bool,
}

impl WorkOrdersMachineListPage {
    pub fn render_table(&self) -> Markup {
        let id_sort = column_sort_url(&self.path_and_query, "Id", &self.sort);
        let id_label = format!("Id{}", sort_indicator(&self.sort, "Id"));
        let name_sort = column_sort_url(&self.path_and_query, "Name", &self.sort);
        let name_label = format!("Name{}", sort_indicator(&self.sort, "Name"));
        let headers = [
            TableColumnHeader {
                key: "Id",
                label: &id_label,
                sort_url: Some(&id_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "Name",
                label: &name_label,
                sort_url: Some(&name_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "Formula",
                label: "Formula",
                sort_url: None,
                push_url: false,
            },
        ];
        let id_labels: Vec<String> = self
            .machines
            .items
            .iter()
            .map(|m| m.id.to_string())
            .collect();
        let rows: Vec<TableRow> = self
            .machines
            .items
            .iter()
            .enumerate()
            .map(|(i, m)| TableRow {
                attrs: row_attr_navigate_route(WorkOrdersMachineDetailRouteTag::new(m.id)),
                cells: vec![
                    field_text(FieldText {
                        value: &id_labels[i],
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &m.name,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &m.formula_label,
                        classes: "font-mono text-xs",
                    }),
                ],
            })
            .collect();
        let mut actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_get_route::<WorkOrdersMachineTableKey, WorkOrdersMachinesRouteTag>(
                        WorkOrdersMachinesRouteTag,
                    ),
                    inputs: MachineFilterForm::render_inputs(
                        &FormCtx::form::<MachineFilterForm>(CsrfToken::current())
                            .value(MachineFilterFormField::Name, &self.filter_name),
                    ),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Apply", ..Default::default() }))
                    },
                    ..Default::default()
                }),
                ..Default::default()
            }))
        };
        if self.can_edit {
            actions = html! {
                (actions)
                (table_create_button::<WorkOrdersMachineTableKey, WorkOrdersMachineCreateModalKey>(
                    Some("plus"),
                    "btn-square btn-outline btn-sm",
                ))
            };
        }
        data_table_list_refresh::<WorkOrdersMachineTableKey>(
            "Machines",
            actions,
            &headers,
            &rows,
            render_pagination::<WorkOrdersMachineTableKey>(
                &self.path_and_query,
                self.machines.number,
                self.machines.num_pages,
            ),
            &self.path_and_query,
        )
    }
}

impl RenderAppPane for WorkOrdersMachineListPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        scaffold_pane(
            wo_menu("machines"),
            machines_list_crumbs(),
            self.render_table(),
        )
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        scaffold_main(machines_list_crumbs(), self.render_table())
    }
}

impl RenderTemplate for WorkOrdersMachineListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Machines — KDS Quotations",
            chrome,
            wo_menu("machines"),
            machines_list_crumbs(),
            self.render_table(),
        )
    }
}

#[derive(Clone)]
pub struct MachineJobRow {
    name: String,
    duration: String,
    progress: i16,
    order: i64,
    detail_href: String,
    id: i64,
}

#[derive(Generic)]
pub struct WorkOrdersMachineDetailPage {
    pub id: i64,
    pub name: String,
    pub formula_label: String,
    pub variables_label: String,
    pub can_edit: bool,
    pub jobs: Vec<MachineJobRow>,
    pub free_on: String,
    pub sort: String,
    pub path_and_query: String,
}

impl WorkOrdersMachineDetailPage {
    pub fn render_jobs_table(&self) -> Markup {
        let id_sort = column_sort_url(&self.path_and_query, "Id", &self.sort);
        let id_label = format!("Id{}", sort_indicator(&self.sort, "Id"));
        let name_sort = column_sort_url(&self.path_and_query, "Name", &self.sort);
        let name_label = format!("Name{}", sort_indicator(&self.sort, "Name"));
        let duration_sort = column_sort_url(&self.path_and_query, "Duration", &self.sort);
        let duration_label = format!("Duration{}", sort_indicator(&self.sort, "Duration"));
        let progress_sort = column_sort_url(&self.path_and_query, "Progress", &self.sort);
        let progress_label = format!("Progress{}", sort_indicator(&self.sort, "Progress"));
        let order_sort = column_sort_url(&self.path_and_query, "Order", &self.sort);
        let order_label = format!("Order{}", sort_indicator(&self.sort, "Order"));
        let headers = [
            TableColumnHeader {
                key: "Id",
                label: &id_label,
                sort_url: Some(&id_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "Name",
                label: &name_label,
                sort_url: Some(&name_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "Duration",
                label: &duration_label,
                sort_url: Some(&duration_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "Progress",
                label: &progress_label,
                sort_url: Some(&progress_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "Order",
                label: &order_label,
                sort_url: Some(&order_sort),
                push_url: true,
            },
        ];
        let id_labels: Vec<String> = self.jobs.iter().map(|j| j.id.to_string()).collect();
        let progress_labels: Vec<String> = self
            .jobs
            .iter()
            .map(|j| format!("{}%", j.progress))
            .collect();
        let order_labels: Vec<String> = self.jobs.iter().map(|j| j.order.to_string()).collect();
        let rows: Vec<TableRow> = self
            .jobs
            .iter()
            .enumerate()
            .map(|(i, job)| TableRow {
                attrs: row_attr_navigate(&job.detail_href),
                cells: vec![
                    field_text(FieldText {
                        value: &id_labels[i],
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &job.name,
                        classes: "",
                    }),
                    field_duration(FieldDuration {
                        value: &job.duration,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &progress_labels[i],
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &order_labels[i],
                        classes: "",
                    }),
                ],
            })
            .collect();
        data_table_list_refresh::<WorkOrdersMachineJobsTableKey>(
            "Jobs",
            html! {},
            &headers,
            &rows,
            html! {},
            &self.path_and_query,
        )
    }

    fn body(&self) -> Markup {
        let actions = if self.can_edit {
            html! {
                (button_modal_form(ButtonModalForm {
                    name: "wo.MachineEditForm",
                    href: &WorkOrdersMachineEditGetRouteTag::new(self.id).url(),
                    form_post_url: &WorkOrdersMachineEditPostRouteTag::new(self.id).path(),
                    modal_uid: WorkOrdersMachineEditModalKey::ID,
                    label: "Edit",
                    classes: "btn-outline",
                    ..Default::default()
                }))
            }
        } else {
            html! {}
        };
        html! {
            (detail_panel(html! {
                (container_column("", html! {
                    (detail_header(DetailHeader {
                        title: &self.name,
                        actions,
                    }))
                    (label("Formula", field_text(FieldText {
                        value: &self.formula_label,
                        classes: "font-mono font-semibold",
                    })))
                    (label("Variables", field_text(FieldText {
                        value: if self.variables_label.is_empty() { "—" } else { &self.variables_label },
                        classes: "font-mono text-sm",
                    })))
                    (label("Free on", field_text(FieldText {
                        value: &self.free_on,
                        classes: "",
                    })))
                    div class="mt-6" {
                        (self.render_jobs_table())
                    }
                }))
            }))
        }
    }
}

impl RenderAppPane for WorkOrdersMachineDetailPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        scaffold_pane(
            wo_menu("machines"),
            machine_crumbs(&self.name, self.id),
            self.body(),
        )
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        scaffold_main(machine_crumbs(&self.name, self.id), self.body())
    }
}

impl RenderTemplate for WorkOrdersMachineDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Machine — KDS Quotations",
            chrome,
            wo_menu("machines"),
            machine_crumbs(&self.name, self.id),
            self.body(),
        )
    }
}

#[derive(Generic)]
pub struct WorkOrdersMachineCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub target_input: String,
    pub name: String,
    pub cost_formula: String,
    pub variables: Vec<String>,
    pub error: String,
}

impl RenderTemplate for WorkOrdersMachineCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        modal_keyed::<WorkOrdersMachineCreateModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "New machine" }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url::<WorkOrdersMachineCreateModalKey>(&modal_create_post_query(
                        WorkOrdersMachineCreatePostRouteTag,
                        &self.form_name,
                        &self.refresh_table,
                        &self.target_input,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: MachineForm::render_inputs(
                        &FormCtx::form::<MachineForm>(CsrfToken::current())
                            .value(MachineFormField::Name, &self.name)
                            .value(MachineFormField::CostFormula, &self.cost_formula)
                            .list(MachineFormField::Variables, &self.variables),
                    ),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Create machine", ..Default::default() }))
                    },
                    ..Default::default()
                }))
            },
        )
    }
}

#[derive(Generic)]
pub struct WorkOrdersMachineEditModalPage {
    pub id: i64,
    pub form_name: String,
    pub name: String,
    pub cost_formula: String,
    pub variables: Vec<String>,
    pub error: String,
}

impl RenderTemplate for WorkOrdersMachineEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let delete_url = WorkOrdersMachineDeleteGetRouteTag::new(self.id).url();
        modal_keyed::<WorkOrdersMachineEditModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit machine" }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url::<WorkOrdersMachineEditModalKey>(&modal_edit_post_url(
                        WorkOrdersMachineEditPostRouteTag::new(self.id),
                        &self.form_name,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: MachineForm::render_inputs(
                        &FormCtx::form::<MachineForm>(CsrfToken::current())
                            .value(MachineFormField::Name, &self.name)
                            .value(MachineFormField::CostFormula, &self.cost_formula)
                            .list(MachineFormField::Variables, &self.variables),
                    ),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Save", ..Default::default() }))
                        (button_modal_form(ButtonModalForm {
                            label: "Delete",
                            icon_name: Some("trash"),
                            name: "wo.MachineDeleteForm",
                            href: &delete_url,
                            form_post_url: &delete_url,
                            modal_uid: WorkOrdersMachineDeleteModalKey::ID,
                            classes: "btn-error",
                            ..Default::default()
                        }))
                    },
                    ..Default::default()
                }))
            },
        )
    }
}

pub async fn list(
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<lariv_core::components::SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<MachineListQuery>,
) -> Markup {
    let machines = load_machine_rows(&state.db, &q, &ctx, DEFAULT_PAGE_SIZE).await;
    let page = WorkOrdersMachineListPage {
        machines,
        filter_name: q.name.clone().unwrap_or_default(),
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        can_edit: can_manage(&ctx),
    };
    let slot_ctx = lariv_core::components::SlotCtx::from_auth(&ctx);
    if htmx.targets::<WorkOrdersMachineTableKey>() {
        return page.render_table();
    }
    html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx)
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
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<lariv_core::components::SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Path(id): Path<i64>,
    Query(q): Query<MachineDetailQuery>,
) -> Response {
    let Some(m) = find_machine_scoped(&state.db, id, &ctx).await else {
        return Redirect::to(&list_url()).into_response();
    };
    let sort = machine_detail_sort(q.sort.as_deref());
    let jobs = load_machine_jobs(&state.db, m.id, Some(&sort)).await;
    let remaining = machine_remaining_duration(&state.db, m.id)
        .await
        .unwrap_or_else(|_| chrono::Duration::zero());
    let free_on = ctx
        .format_datetime(machine_free_on(Utc::now(), remaining))
        .into_string();
    let page = WorkOrdersMachineDetailPage {
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
    if htmx.targets::<WorkOrdersMachineJobsTableKey>() {
        return page.render_jobs_table().into_response();
    }
    html_built_page_or_app_layout(
        &page,
        &htmx,
        &chrome,
        &lariv_core::components::SlotCtx::from_auth(&ctx),
    )
    .into_response()
}

pub async fn create_get(
    Cap(chrome): Cap<lariv_core::components::SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalFormQuery>,
) -> Markup {
    if !can_manage(&ctx) {
        return html! { div class="alert alert-error" { "Forbidden" } };
    }
    let page = WorkOrdersMachineCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        target_input: q.target_input(),
        name: String::new(),
        cost_formula: String::new(),
        variables: Vec::new(),
        error: String::new(),
    };
    html_built_page_with_slots(
        &page,
        &chrome,
        &lariv_core::components::SlotCtx::from_auth(&ctx),
    )
}

pub async fn create_post(
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<lariv_core::components::SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalFormQuery>,
    lariv_core::html_form::HtmlFormBody(form): lariv_core::html_form::HtmlFormBody<MachineForm>,
) -> Response {
    if !can_manage(&ctx) {
        return Redirect::to(&list_url()).into_response();
    }
    let slot_ctx = lariv_core::components::SlotCtx::from_auth(&ctx);
    let parsed = parse_machine_form(&form);
    let (name, cost_formula, variables) = match parsed {
        Ok(v) => v,
        Err(error) => {
            let page = WorkOrdersMachineCreateModalPage {
                form_name: q.form_name(),
                refresh_table: q.refresh_table(),
                target_input: q.target_input(),
                name: form.name,
                cost_formula: form.cost_formula,
                variables: form.variables,
                error,
            };
            return html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response();
        }
    };
    let now = Utc::now();
    let model = machine::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        name: Set(name),
        cost_formula: Set(cost_formula),
        variables: Set(variables),
    };
    match model.insert(&state.db).await {
        Ok(saved) => respond_create_modal_done_fk::<WorkOrdersMachineCreateModalKey>(
            &htmx,
            &q.refresh_table(),
            &WorkOrdersMachineDetailRouteTag::new(saved.id).url(),
            saved.id,
            &saved.name,
            &q.target_input(),
        ),
        Err(e) => {
            let page = WorkOrdersMachineCreateModalPage {
                form_name: q.form_name(),
                refresh_table: q.refresh_table(),
                target_input: q.target_input(),
                name: form.name,
                cost_formula: form.cost_formula,
                variables: form.variables,
                error: e.to_string(),
            };
            html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response()
        }
    }
}

pub async fn edit_get(
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<lariv_core::components::SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalFormQuery>,
) -> Response {
    if !can_manage(&ctx) {
        return Redirect::to(&list_url()).into_response();
    }
    let Some(m) = find_machine_scoped(&state.db, id, &ctx).await else {
        return Redirect::to(&list_url()).into_response();
    };
    let variables = schema_entries_for(&m);
    let page = WorkOrdersMachineEditModalPage {
        id: m.id,
        form_name: q.form_name(),
        name: m.name,
        cost_formula: m.cost_formula,
        variables,
        error: String::new(),
    };
    html_built_page_with_slots(
        &page,
        &chrome,
        &lariv_core::components::SlotCtx::from_auth(&ctx),
    )
    .into_response()
}

pub async fn edit_post(
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<lariv_core::components::SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Query(q): Query<ModalFormQuery>,
    lariv_core::html_form::HtmlFormBody(form): lariv_core::html_form::HtmlFormBody<MachineForm>,
) -> Response {
    if !can_manage(&ctx) {
        return Redirect::to(&list_url()).into_response();
    }
    let Some(existing) = find_machine_scoped(&state.db, id, &ctx).await else {
        return Redirect::to(&list_url()).into_response();
    };
    let slot_ctx = lariv_core::components::SlotCtx::from_auth(&ctx);
    let parsed = parse_machine_form(&form);
    let (name, cost_formula, variables) = match parsed {
        Ok(v) => v,
        Err(error) => {
            let page = WorkOrdersMachineEditModalPage {
                id,
                form_name: q.form_name(),
                name: form.name,
                cost_formula: form.cost_formula,
                variables: form.variables,
                error,
            };
            return html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response();
        }
    };
    let now = Utc::now();
    let mut am: machine::ActiveModel = existing.into();
    am.updated_at = Set(Some(now));
    am.name = Set(name);
    am.cost_formula = Set(cost_formula);
    am.variables = Set(variables);
    match am.update(&state.db).await {
        Ok(_) => respond_edit_modal_done::<WorkOrdersMachineEditModalKey>(
            &htmx,
            &WorkOrdersMachineDetailRouteTag::new(id).url(),
        ),
        Err(e) => {
            let page = WorkOrdersMachineEditModalPage {
                id,
                form_name: q.form_name(),
                name: form.name,
                cost_formula: form.cost_formula,
                variables: form.variables,
                error: e.to_string(),
            };
            html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response()
        }
    }
}

pub async fn delete_get(
    Cap(chrome): Cap<lariv_core::components::SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
) -> Markup {
    let page = ConfirmDeleteModalPage {
        modal_uid: WorkOrdersMachineDeleteModalKey::ID.to_string(),
        title: "Delete machine".into(),
        message: "Are you sure you want to delete this machine?".into(),
        post_url: WorkOrdersMachineDeletePostRouteTag::new(id).url(),
        error: String::new(),
    };
    html_built_page_with_slots(
        &page,
        &chrome,
        &lariv_core::components::SlotCtx::from_auth(&ctx),
    )
}

pub async fn delete_post(
    Cap(state): Cap<WorkOrdersState>,
    Cap(chrome): Cap<lariv_core::components::SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    if !can_manage(&ctx) {
        return Redirect::to(&list_url()).into_response();
    }
    match MachineEntity::delete_by_id(id).exec(&state.db).await {
        Ok(_) => htmx.redirect(&list_url()),
        Err(e) => {
            tracing::error!(error = %e, id, "failed to delete machine");
            let page = ConfirmDeleteModalPage {
                modal_uid: WorkOrdersMachineDeleteModalKey::ID.to_string(),
                title: "Delete machine".into(),
                message: "Are you sure you want to delete this machine?".into(),
                post_url: WorkOrdersMachineDeletePostRouteTag::new(id).url(),
                error: e.to_string(),
            };
            html_built_page_with_slots(
                &page,
                &chrome,
                &lariv_core::components::SlotCtx::from_auth(&ctx),
            )
            .into_response()
        }
    }
}
