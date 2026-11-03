//! Work order list, detail, and modals.

use maud::{Markup, html};

use lariv_core::components::{
    ButtonModalForm, ButtonSubmit, DeleteConfirmation, FieldLink, FieldText, FormOpts, Modal,
    ObjectList, PaginationPage, ShellChrome, SlotCapability, SlotRegistrar,
    TableColumnHeader, TablePagination, TableRow, button_modal_form, button_submit,
    data_table_list, delete_confirmation, detail, detail_header, field_link, field_text, form,
    form_hx_post_selector, form_hx_post_url, modal, modal_keyed, pagination_pages,
    row_attr_navigate_route, table_create_button, table_pagination,
};
use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm};
use lariv_core::http::ProvideRequestCaps;
use lariv_core::template::{
    RenderAppPane, RenderTemplate, TemplateCapability, TemplateOf, TemplateRegistrar,
};
use lariv_core::web::{modal_create_post_url, modal_edit_post_url};

use super::crumbs::{self, detail_url};
use super::entities::{part, step};
use super::forms::{
    FlowForm, FlowFormField, PartForm, PartFormField, PresuppliedForm, PresuppliedFormField,
    StepForm, StepFormField, WorkOrderForm, WorkOrderFormField,
};
use super::create_modals::WORK_ORDER_CREATE;
use super::graph::{self, PartRole, PlanLayout};
use super::keys::*;
use super::logic::{FlowSide, Plan};
use super::routes::*;

lariv_core::define_register_items! {
    plugin: super::WorkOrdersTag;
    capability: TemplateCapability;
    trait: TemplateRegistrar;
    method: register_templates;
    wrapper: TemplateOf;
    bounds: [Clone, ProvideRequestCaps, Send, Sync];
    hook: Hook;
    items: [
        WorkOrderListPageIdx: WorkOrderListPageTag => WorkOrderListPage,
        WorkOrderDetailPageIdx: WorkOrderDetailPageTag => WorkOrderDetailPage,
        WorkOrderCreateModalPageIdx: WorkOrderCreateModalPageTag => WorkOrderCreateModalPage,
        WorkOrderEditModalPageIdx: WorkOrderEditModalPageTag => WorkOrderEditModalPage,
        PartModalPageIdx: PartModalPageTag => PartModalPage,
        PresuppliedModalPageIdx: PresuppliedModalPageTag => PresuppliedModalPage,
        StepModalPageIdx: StepModalPageTag => StepModalPage,
        FlowModalPageIdx: FlowModalPageTag => FlowModalPage,
        CompleteModalPageIdx: CompleteModalPageTag => CompleteModalPage,
        ConfirmDeleteModalPageIdx: ConfirmDeleteModalPageTag => ConfirmDeleteModalPage,
    ]
}

lariv_core::define_register_items! {
    plugin: super::WorkOrdersTag;
    capability: SlotCapability;
    trait: SlotRegistrar;
    method: register_slots;
    bounds: [];
    items: [];
    hook: SlotsHook;
}

fn app_scaffold(chrome: &ShellChrome, crumbs: Markup, body: Markup) -> Markup {
    lariv_core::components::shell_scaffold(lariv_core::components::ShellScaffold {
        title: "Work orders",
        registry_head: chrome.head.clone(),
        topbar_items: chrome.topbar_items.clone(),
        right_sidebar: chrome.right_sidebar.clone(),
        sidebar: crumbs::menu(),
        breadcrumbs: crumbs,
        body,
        ..Default::default()
    })
}

fn scaffold_pane(crumbs: Markup, body: Markup) -> lariv_core::components::AppLayoutHtml {
    lariv_core::components::layout_sidebar(lariv_core::components::LayoutSidebar {
        sidebar: crumbs::menu(),
        breadcrumbs: crumbs,
        content: body,
    })
}

fn scaffold_main(crumbs: Markup, body: Markup) -> lariv_core::components::MainContentHtml {
    lariv_core::components::layout_main(lariv_core::components::LayoutMain {
        breadcrumbs: crumbs,
        content: body,
    })
}

fn render_pagination(path_and_query: &str, number: u32, num_pages: u32) -> Markup {
    let hx_target = WORK_ORDER_TABLE.selector();
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
        hx_target: &hx_target,
    })
}

pub struct WorkOrderRow {
    pub id: i64,
    pub title: String,
    pub customer: String,
    pub progress: String,
}

pub struct WorkOrderListPage {
    pub orders: ObjectList<WorkOrderRow>,
    pub path_and_query: String,
}

impl WorkOrderListPage {
    pub fn render_table(&self) -> Markup {
        let headers = [
            TableColumnHeader {
                key: "title",
                label: "Title",
                sort_url: None,
                push_url: false,
            },
            TableColumnHeader {
                key: "customer",
                label: "Customer",
                sort_url: None,
                push_url: false,
            },
            TableColumnHeader {
                key: "progress",
                label: "Progress",
                sort_url: None,
                push_url: false,
            },
        ];
        let rows: Vec<TableRow> = self
            .orders
            .items
            .iter()
            .map(|order| TableRow {
                attrs: row_attr_navigate_route(WorkOrderDetailRouteTag::new(order.id)),
                cells: vec![
                    field_text(FieldText {
                        value: &order.title,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &order.customer,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &order.progress,
                        classes: "",
                    }),
                ],
                key: String::new(),
                children: Vec::new(),
            })
            .collect();
        let actions = table_create_button(WORK_ORDER_TABLE, WORK_ORDER_CREATE, 
            Some("plus"),
            "btn-primary btn-sm",
        );
        data_table_list(WORK_ORDER_TABLE, 
            "Work orders",
            actions,
            &headers,
            &rows,
            render_pagination(
                &self.path_and_query,
                self.orders.number,
                self.orders.num_pages,
            ),
        )
    }
}

impl RenderAppPane for WorkOrderListPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        scaffold_pane(crumbs::list_crumbs(), self.render_table())
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        scaffold_main(crumbs::list_crumbs(), self.render_table())
    }
}

impl RenderTemplate for WorkOrderListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(chrome, crumbs::list_crumbs(), self.render_table())
    }
}

pub struct WorkOrderDetailPage {
    pub plan: Plan,
    pub customer: String,
    pub customer_href: String,
    pub files_href: String,
    pub files_label: String,
    pub layout: Option<PlanLayout>,
    pub layout_error: String,
}

impl WorkOrderDetailPage {
    pub fn render_graph(&self) -> Markup {
        let body = match &self.layout {
            Some(layout) => graph_body(&self.plan, layout),
            None => html! { p class="text-error" { (self.layout_error) } },
        };
        html! {
            div id=(WORK_ORDER_GRAPH.id()) class="flex flex-col gap-3" {
                (body)
            }
        }
    }

    fn render_body(&self) -> Markup {
        let id = self.plan.order.id;
        let edit_url = WorkOrderEditGetRouteTag::new(id).url();
        html! {
            div class="flex flex-col gap-6" {
                (detail(html! {
                    (detail_header(lariv_core::components::DetailHeader {
                        title: &self.plan.order.title,
                        actions: html! {
                            (button_modal_form(ButtonModalForm {
                                label: "Edit",
                                icon_name: Some("pencil"),
                                name: "wo.WorkOrderEditForm",
                                href: &edit_url,
                                form_post_url: &edit_url,
                                modal_uid: WORK_ORDER_EDIT_MODAL.id(),
                                classes: "btn-sm",
                                ..Default::default()
                            }))
                        },
                    }))
                    div class="mt-2 flex flex-col gap-1" {
                        (field_link(FieldLink { href: &self.customer_href, label: &self.customer, classes: "" }))
                        (field_link(FieldLink { href: &self.files_href, label: &self.files_label, classes: "" }))
                    }
                }))
                section class="flex flex-col gap-2" {
                    h3 class="font-semibold" { "Plan" }
                    (self.render_graph())
                }
                (parts_section(&self.plan))
                (presupplied_section(&self.plan))
                (steps_section(&self.plan))
            }
        }
    }
}

impl RenderAppPane for WorkOrderDetailPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        scaffold_pane(
            crumbs::detail_crumbs(&self.plan.order.title),
            self.render_body(),
        )
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        scaffold_main(
            crumbs::detail_crumbs(&self.plan.order.title),
            self.render_body(),
        )
    }
}

impl RenderTemplate for WorkOrderDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            chrome,
            crumbs::detail_crumbs(&self.plan.order.title),
            self.render_body(),
        )
    }
}

fn parts_section(plan: &Plan) -> Markup {
    html! {
        section class="flex flex-col gap-2" {
            h3 class="font-semibold" { "Parts" }
            table class="table table-sm" {
                thead { tr { th { "Name" } th { "Kind" } th { "Unit" } } }
                tbody {
                    @for part in &plan.parts {
                        tr {
                            td { (part.name) }
                            td { (part.measure_kind) }
                            td { (part.measure_unit) }
                        }
                    }
                }
            }
        }
    }
}

fn presupplied_section(plan: &Plan) -> Markup {
    html! {
        section class="flex flex-col gap-2" {
            h3 class="font-semibold" { "Presupplied parts" }
            table class="table table-sm" {
                thead { tr { th { "Part" } th { "Quantity" } } }
                tbody {
                    @for line in &plan.presupplied {
                        tr {
                            td { (part_name(&plan.parts, line.part_id)) }
                            td { (format!("{} {}", line.qty_value.normalize(), line.qty_unit)) }
                        }
                    }
                }
            }
        }
    }
}

fn steps_section(plan: &Plan) -> Markup {
    html! {
        section class="flex flex-col gap-4" {
            h3 class="font-semibold" { "Steps" }
            @for step in &plan.steps {
                (step_card(plan, step))
            }
        }
    }
}

fn step_card(plan: &Plan, step: &step::Model) -> Markup {
    let done = plan
        .completions
        .iter()
        .filter(|row| row.step_id == step.id)
        .count();
    let complete_url = CompleteGetRouteTag::new(step.id).url();
    html! {
        article class="rounded-box border border-base-300 p-3 flex flex-col gap-3" {
            div class="flex flex-wrap items-center justify-between gap-2" {
                div {
                    h4 class="font-medium" { (step.name) }
                    p class="text-sm opacity-70" { (format!("{done}/{} batches", step.planned_batches)) }
                }
                (button_modal_form(ButtonModalForm {
                    label: "Record batch",
                    icon_name: Some("check"),
                    name: "wo.CompleteForm",
                    href: &complete_url,
                    form_post_url: &complete_url,
                    modal_uid: COMPLETE_MODAL.id(),
                    classes: "btn-sm btn-primary",
                    ..Default::default()
                }))
            }
            div class="grid gap-3 md:grid-cols-2" {
                (flow_table(plan, step.id, FlowSide::Input))
                (flow_table(plan, step.id, FlowSide::Output))
            }
            (completion_list(plan, step.id))
        }
    }
}

fn flow_table(plan: &Plan, step_id: i64, side: FlowSide) -> Markup {
    let title = match side {
        FlowSide::Input => "Inputs per batch",
        FlowSide::Output => "Outputs per batch",
    };
    html! {
        div class="flex flex-col gap-1" {
            h5 class="text-sm font-medium" { (title) }
            ul class="text-sm" {
                @match side {
                    FlowSide::Input => {
                        @for line in plan.inputs.iter().filter(|line| line.step_id == step_id) {
                            li { (format!("{} {} {}", part_name(&plan.parts, line.part_id), line.qty_value.normalize(), line.qty_unit)) }
                        }
                    }
                    FlowSide::Output => {
                        @for line in plan.outputs.iter().filter(|line| line.step_id == step_id) {
                            li { (format!("{} {} {}", part_name(&plan.parts, line.part_id), line.qty_value.normalize(), line.qty_unit)) }
                        }
                    }
                }
            }
        }
    }
}

fn completion_list(plan: &Plan, step_id: i64) -> Markup {
    let mut rows: Vec<_> = plan
        .completions
        .iter()
        .filter(|row| row.step_id == step_id)
        .collect();
    rows.sort_by_key(|row| row.completed_at);
    html! {
        div class="flex flex-col gap-1" {
            h5 class="text-sm font-medium" { "Recorded batches" }
            @if rows.is_empty() {
                p class="text-sm opacity-60" { "No batches recorded." }
            }
            ul class="text-sm flex flex-col gap-1" {
                @for (index, row) in rows.iter().enumerate() {
                    li class="flex flex-wrap items-center justify-between gap-2" {
                        span { (format!("Batch {} · {}", index.saturating_add(1), row.completed_at.format("%Y-%m-%d %H:%M"))) }
                        (edit_button("Delete", &CompletionDeleteGetRouteTag::new(row.id).url(), COMPLETION_DELETE_MODAL.id(), "wo.CompletionDeleteForm"))
                    }
                }
            }
        }
    }
}

fn edit_button(label: &str, href: &str, modal_uid: &str, name: &str) -> Markup {
    button_modal_form(ButtonModalForm {
        label,
        icon_name: None,
        name,
        href,
        form_post_url: href,
        modal_uid,
        classes: "btn-xs",
        ..Default::default()
    })
}

fn part_name(parts: &[part::Model], id: i64) -> &str {
    parts
        .iter()
        .find(|part| part.id == id)
        .map(|part| part.name.as_str())
        .unwrap_or("Missing part")
}

const NODE_W: i32 = 112;
const NODE_H: i32 = 32;
const COL_STEP: i32 = 124;
const RANK_STEP: i32 = 44;

fn graph_body(plan: &Plan, layout: &PlanLayout) -> Markup {
    let placed = place_nodes(plan, layout);
    let width = placed
        .iter()
        .map(|node| node.x.saturating_add(NODE_W).saturating_add(8))
        .max()
        .unwrap_or(160);
    let height = placed
        .iter()
        .map(|node| node.y.saturating_add(NODE_H).saturating_add(8))
        .max()
        .unwrap_or(NODE_H);
    let unused: Vec<_> = plan
        .parts
        .iter()
        .filter(|part| graph::role_of(layout, part.id) == Some(PartRole::Unused))
        .collect();
    html! {
        div class="flex flex-col gap-3 lg:flex-row lg:items-start" {
            div class="min-w-0 overflow-auto max-h-52" {
                @if placed.is_empty() {
                    p class="text-sm opacity-70" { "Connect parts with steps to draw the plan." }
                } @else {
                    svg xmlns="http://www.w3.org/2000/svg" viewBox=(format!("0 0 {width} {height}")) width=(width) height=(height) class="max-w-full h-auto text-base-content" role="img" aria-label="Work order plan" {
                        @for edge in edges(plan, &placed) {
                            line x1=(edge.0) y1=(edge.1) x2=(edge.2) y2=(edge.3) stroke="currentColor" stroke-opacity="0.45" {}
                        }
                        @for node in &placed {
                            rect x=(node.x) y=(node.y) width=(NODE_W) height=(NODE_H) rx="6" fill="currentColor" fill-opacity=(node.fill) stroke="currentColor" {}
                            text x=(node.x.saturating_add(NODE_W / 2)) y=(node.y.saturating_add(13)) text-anchor="middle" font-size="11" fill="currentColor" { (node.title) }
                            text x=(node.x.saturating_add(NODE_W / 2)) y=(node.y.saturating_add(25)) text-anchor="middle" font-size="10" fill="currentColor" opacity="0.7" { (node.subtitle) }
                        }
                    }
                }
            }
            div class="w-full lg:w-56" {
                h4 class="text-sm font-medium" { "Unused parts" }
                @if unused.is_empty() {
                    p class="text-sm opacity-60" { "None" }
                } @else {
                    ul class="text-sm" {
                        @for part in unused {
                            li { (part.name) }
                        }
                    }
                }
            }
        }
        ul class="text-sm flex flex-col gap-1" {
            @for row in &layout.coverage {
                li class=(if row.gap.is_sign_negative() { "text-error" } else { "" }) {
                    (format!(
                        "{}: supply {}, demand {}, gap {}",
                        part_name(&plan.parts, row.part_id),
                        row.supply.normalize(),
                        row.demand.normalize(),
                        row.gap.normalize()
                    ))
                }
            }
        }
    }
}

struct Placed {
    kind: NodeKind,
    id: i64,
    x: i32,
    y: i32,
    title: String,
    subtitle: String,
    fill: &'static str,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum NodeKind {
    Part,
    Step,
}

fn place_nodes(plan: &Plan, layout: &PlanLayout) -> Vec<Placed> {
    let mut ranks: Vec<u32> = Vec::new();
    for node in &layout.part_ranks {
        if graph::role_of(layout, node.id) != Some(PartRole::Unused) && !ranks.contains(&node.rank)
        {
            ranks.push(node.rank);
        }
    }
    for node in &layout.step_ranks {
        if !ranks.contains(&node.rank) {
            ranks.push(node.rank);
        }
    }
    ranks.sort_unstable();
    let mut placed = Vec::new();
    for rank in ranks {
        let mut column = 0i32;
        for node in layout.part_ranks.iter().filter(|node| node.rank == rank) {
            if graph::role_of(layout, node.id) == Some(PartRole::Unused) {
                continue;
            }
            let name = part_name(&plan.parts, node.id).to_string();
            let role = graph::role_of(layout, node.id)
                .map(PartRole::label)
                .unwrap_or("");
            placed.push(Placed {
                kind: NodeKind::Part,
                id: node.id,
                x: column.saturating_mul(COL_STEP).saturating_add(8),
                y: i32::try_from(rank)
                    .unwrap_or(0)
                    .saturating_mul(RANK_STEP)
                    .saturating_add(8),
                title: clip_label(&name),
                subtitle: role.to_string(),
                fill: "0.04",
            });
            column = column.saturating_add(1);
        }
        for node in layout.step_ranks.iter().filter(|node| node.rank == rank) {
            let name = plan
                .steps
                .iter()
                .find(|step| step.id == node.id)
                .map(|step| step.name.as_str())
                .unwrap_or("Step");
            let done = plan
                .completions
                .iter()
                .filter(|row| row.step_id == node.id)
                .count();
            let planned = plan
                .steps
                .iter()
                .find(|step| step.id == node.id)
                .map(|step| step.planned_batches)
                .unwrap_or(0);
            placed.push(Placed {
                kind: NodeKind::Step,
                id: node.id,
                x: column.saturating_mul(COL_STEP).saturating_add(8),
                y: i32::try_from(rank)
                    .unwrap_or(0)
                    .saturating_mul(RANK_STEP)
                    .saturating_add(8),
                title: clip_label(name),
                subtitle: format!("{done}/{planned}"),
                fill: "0.12",
            });
            column = column.saturating_add(1);
        }
    }
    placed
}

fn edges(plan: &Plan, placed: &[Placed]) -> Vec<(i32, i32, i32, i32)> {
    let mut lines = Vec::new();
    for flow in &plan.inputs {
        if let (Some(from), Some(to)) = (
            center(placed, NodeKind::Part, flow.part_id),
            center(placed, NodeKind::Step, flow.step_id),
        ) {
            lines.push((from.0, from.1, to.0, to.1));
        }
    }
    for flow in &plan.outputs {
        if let (Some(from), Some(to)) = (
            center(placed, NodeKind::Step, flow.step_id),
            center(placed, NodeKind::Part, flow.part_id),
        ) {
            lines.push((from.0, from.1, to.0, to.1));
        }
    }
    lines
}

fn center(placed: &[Placed], kind: NodeKind, id: i64) -> Option<(i32, i32)> {
    placed
        .iter()
        .find(|node| node.kind == kind && node.id == id)
        .map(|node| {
            (
                node.x.saturating_add(NODE_W / 2),
                node.y.saturating_add(NODE_H / 2),
            )
        })
}

fn clip_label(value: &str) -> String {
    let mut clipped = String::new();
    for (index, ch) in value.chars().enumerate() {
        if index == 22 {
            clipped.push('…');
            break;
        }
        clipped.push(ch);
    }
    clipped
}

fn submit(label: &str) -> Markup {
    button_submit(ButtonSubmit {
        label,
        classes: "btn-primary",
        ..Default::default()
    })
}

pub struct WorkOrderCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub title: String,
    pub customer_id: String,
    pub customer_display: String,
    pub folder_vnode_id: String,
    pub folder_display: String,
    pub parts_json: String,
    pub presupplied_json: String,
    pub steps_json: String,
    pub error: String,
}

impl RenderTemplate for WorkOrderCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let ctx = order_form_ctx(OrderFields {
            title: &self.title,
            customer_id: &self.customer_id,
            customer_display: &self.customer_display,
            folder_vnode_id: &self.folder_vnode_id,
            folder_display: &self.folder_display,
            parts_json: &self.parts_json,
            presupplied_json: &self.presupplied_json,
            steps_json: &self.steps_json,
        });
        modal_keyed(WORK_ORDER_CREATE_MODAL, 
            "",
            form(
                &CsrfToken::current(),
                FormOpts {
                    title: "New work order",
                    attrs: form_hx_post_url(WORK_ORDER_CREATE_MODAL, &modal_create_post_url(
                        WorkOrderCreatePostRouteTag,
                        &self.form_name,
                        &self.refresh_table,
                    )),
                    form_error: Some(self.error.as_str()).filter(|err| !err.is_empty()),
                    inputs: WorkOrderForm::render_inputs(&ctx),
                    actions: submit("Create"),
                    ..Default::default()
                },
            ),
        )
    }
}

pub struct WorkOrderEditModalPage {
    pub id: i64,
    pub form_name: String,
    pub title: String,
    pub customer_id: String,
    pub customer_display: String,
    pub folder_vnode_id: String,
    pub folder_display: String,
    pub parts_json: String,
    pub presupplied_json: String,
    pub steps_json: String,
    pub error: String,
}

impl RenderTemplate for WorkOrderEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let ctx = order_form_ctx(OrderFields {
            title: &self.title,
            customer_id: &self.customer_id,
            customer_display: &self.customer_display,
            folder_vnode_id: &self.folder_vnode_id,
            folder_display: &self.folder_display,
            parts_json: &self.parts_json,
            presupplied_json: &self.presupplied_json,
            steps_json: &self.steps_json,
        });
        let delete_url = WorkOrderDeleteGetRouteTag::new(self.id).url();
        modal_keyed(WORK_ORDER_EDIT_MODAL, 
            "",
            form(
                &CsrfToken::current(),
                FormOpts {
                    title: "Edit work order",
                    attrs: form_hx_post_url(WORK_ORDER_EDIT_MODAL, &modal_edit_post_url(
                        WorkOrderEditPostRouteTag::new(self.id),
                        &self.form_name,
                    )),
                    form_error: Some(self.error.as_str()).filter(|err| !err.is_empty()),
                    inputs: WorkOrderForm::render_inputs(&ctx),
                    actions: html! {
                        (submit("Save"))
                        (button_modal_form(ButtonModalForm {
                            label: "Delete",
                            icon_name: Some("trash"),
                            name: "wo.WorkOrderDeleteForm",
                            href: &delete_url,
                            form_post_url: &delete_url,
                            modal_uid: WORK_ORDER_DELETE_MODAL.id(),
                            classes: "btn-error",
                            ..Default::default()
                        }))
                    },
                    ..Default::default()
                },
            ),
        )
    }
}

struct OrderFields<'a> {
    title: &'a str,
    customer_id: &'a str,
    customer_display: &'a str,
    folder_vnode_id: &'a str,
    folder_display: &'a str,
    parts_json: &'a str,
    presupplied_json: &'a str,
    steps_json: &'a str,
}

fn order_form_ctx(fields: OrderFields<'_>) -> FormCtx<'_> {
    FormCtx::form::<WorkOrderForm>(CsrfToken::current())
        .value(WorkOrderFormField::Title, fields.title)
        .value(WorkOrderFormField::CustomerId, fields.customer_id)
        .display(WorkOrderFormField::CustomerId, fields.customer_display)
        .value(WorkOrderFormField::FolderVnodeId, fields.folder_vnode_id)
        .display(WorkOrderFormField::FolderVnodeId, fields.folder_display)
        .value(WorkOrderFormField::PartsLinesJson, fields.parts_json)
        .value(
            WorkOrderFormField::PresuppliedLinesJson,
            fields.presupplied_json,
        )
        .value(WorkOrderFormField::StepsLinesJson, fields.steps_json)
        .into()
}

pub struct PartModalPage {
    pub work_order_id: i64,
    pub existing_id: Option<i64>,
    pub form_name: String,
    pub name: String,
    pub measure_kind: String,
    pub measure_unit: String,
    pub kind_choices: Vec<(String, String)>,
    pub error: String,
}

impl RenderTemplate for PartModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let ctx = FormCtx::form::<PartForm>(CsrfToken::current())
            .value(PartFormField::Name, &self.name)
            .value(PartFormField::MeasureKind, &self.measure_kind)
            .value(PartFormField::MeasureUnit, &self.measure_unit)
            .choices(PartFormField::MeasureKind, &self.kind_choices);
        let (title, attrs) = match self.existing_id {
            Some(id) => (
                "Edit part",
                form_hx_post_url(PART_EDIT_MODAL, &modal_edit_post_url(
                    PartEditPostRouteTag::new(id),
                    &self.form_name,
                )),
            ),
            None => (
                "Add part",
                form_hx_post_url(PART_CREATE_MODAL, &modal_create_post_url(
                    PartCreatePostRouteTag::new(self.work_order_id),
                    &self.form_name,
                    "",
                )),
            ),
        };
        let body = form(
            &CsrfToken::current(),
            FormOpts {
                title,
                attrs,
                form_error: Some(self.error.as_str()).filter(|err| !err.is_empty()),
                inputs: PartForm::render_inputs(&ctx),
                actions: html! {
                    (submit("Save"))
                    @if let Some(id) = self.existing_id {
                        (button_modal_form(ButtonModalForm {
                            label: "Delete",
                            icon_name: Some("trash"),
                            name: "wo.PartDeleteForm",
                            href: &PartDeleteGetRouteTag::new(id).url(),
                            form_post_url: &PartDeleteGetRouteTag::new(id).url(),
                            modal_uid: PART_DELETE_MODAL.id(),
                            classes: "btn-error",
                            ..Default::default()
                        }))
                    }
                },
                ..Default::default()
            },
        );
        match self.existing_id {
            Some(_) => modal_keyed(PART_EDIT_MODAL, "", body),
            None => modal_keyed(PART_CREATE_MODAL, "", body),
        }
    }
}

pub struct PresuppliedModalPage {
    pub work_order_id: i64,
    pub existing_id: Option<i64>,
    pub form_name: String,
    pub part_id: String,
    pub qty_value: String,
    pub qty_unit: String,
    pub part_choices: Vec<(String, String)>,
    pub error: String,
}

impl RenderTemplate for PresuppliedModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let ctx = FormCtx::form::<PresuppliedForm>(CsrfToken::current())
            .value(PresuppliedFormField::PartId, &self.part_id)
            .value(PresuppliedFormField::QtyValue, &self.qty_value)
            .value(PresuppliedFormField::QtyUnit, &self.qty_unit)
            .choices(PresuppliedFormField::PartId, &self.part_choices);
        let (title, attrs) = match self.existing_id {
            Some(id) => (
                "Edit supply",
                form_hx_post_url(PRESUPPLIED_EDIT_MODAL, &modal_edit_post_url(
                    PresuppliedEditPostRouteTag::new(id),
                    &self.form_name,
                )),
            ),
            None => (
                "Add supply",
                form_hx_post_url(PRESUPPLIED_CREATE_MODAL, &modal_create_post_url(
                    PresuppliedCreatePostRouteTag::new(self.work_order_id),
                    &self.form_name,
                    "",
                )),
            ),
        };
        let body = form(
            &CsrfToken::current(),
            FormOpts {
                title,
                attrs,
                form_error: Some(self.error.as_str()).filter(|err| !err.is_empty()),
                inputs: PresuppliedForm::render_inputs(&ctx),
                actions: html! {
                    (submit("Save"))
                    @if let Some(id) = self.existing_id {
                        (button_modal_form(ButtonModalForm {
                            label: "Delete",
                            icon_name: Some("trash"),
                            name: "wo.PresuppliedDeleteForm",
                            href: &PresuppliedDeleteGetRouteTag::new(id).url(),
                            form_post_url: &PresuppliedDeleteGetRouteTag::new(id).url(),
                            modal_uid: PRESUPPLIED_DELETE_MODAL.id(),
                            classes: "btn-error",
                            ..Default::default()
                        }))
                    }
                },
                ..Default::default()
            },
        );
        match self.existing_id {
            Some(_) => modal_keyed(PRESUPPLIED_EDIT_MODAL, "", body),
            None => modal_keyed(PRESUPPLIED_CREATE_MODAL, "", body),
        }
    }
}

pub struct StepModalPage {
    pub work_order_id: i64,
    pub existing_id: Option<i64>,
    pub form_name: String,
    pub name: String,
    pub planned_batches: String,
    pub error: String,
}

impl RenderTemplate for StepModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let ctx = FormCtx::form::<StepForm>(CsrfToken::current())
            .value(StepFormField::Name, &self.name)
            .value(StepFormField::PlannedBatches, &self.planned_batches);
        let (title, attrs) = match self.existing_id {
            Some(id) => (
                "Edit step",
                form_hx_post_url(STEP_EDIT_MODAL, &modal_edit_post_url(
                    StepEditPostRouteTag::new(id),
                    &self.form_name,
                )),
            ),
            None => (
                "Add step",
                form_hx_post_url(STEP_CREATE_MODAL, &modal_create_post_url(
                    StepCreatePostRouteTag::new(self.work_order_id),
                    &self.form_name,
                    "",
                )),
            ),
        };
        let body = form(
            &CsrfToken::current(),
            FormOpts {
                title,
                attrs,
                form_error: Some(self.error.as_str()).filter(|err| !err.is_empty()),
                inputs: StepForm::render_inputs(&ctx),
                actions: html! {
                    (submit("Save"))
                    @if let Some(id) = self.existing_id {
                        (button_modal_form(ButtonModalForm {
                            label: "Delete",
                            icon_name: Some("trash"),
                            name: "wo.StepDeleteForm",
                            href: &StepDeleteGetRouteTag::new(id).url(),
                            form_post_url: &StepDeleteGetRouteTag::new(id).url(),
                            modal_uid: STEP_DELETE_MODAL.id(),
                            classes: "btn-error",
                            ..Default::default()
                        }))
                    }
                },
                ..Default::default()
            },
        );
        match self.existing_id {
            Some(_) => modal_keyed(STEP_EDIT_MODAL, "", body),
            None => modal_keyed(STEP_CREATE_MODAL, "", body),
        }
    }
}

pub struct FlowModalPage {
    pub step_id: i64,
    pub existing_id: Option<i64>,
    pub side: FlowSide,
    pub form_name: String,
    pub part_id: String,
    pub qty_value: String,
    pub qty_unit: String,
    pub part_choices: Vec<(String, String)>,
    pub error: String,
}

impl RenderTemplate for FlowModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let ctx = FormCtx::form::<FlowForm>(CsrfToken::current())
            .value(FlowFormField::PartId, &self.part_id)
            .value(FlowFormField::QtyValue, &self.qty_value)
            .value(FlowFormField::QtyUnit, &self.qty_unit)
            .choices(FlowFormField::PartId, &self.part_choices);
        let title = match (self.side, self.existing_id) {
            (FlowSide::Input, None) => "Add input",
            (FlowSide::Input, Some(_)) => "Edit input",
            (FlowSide::Output, None) => "Add output",
            (FlowSide::Output, Some(_)) => "Edit output",
        };
        let attrs = match (self.side, self.existing_id) {
            (FlowSide::Input, Some(id)) => form_hx_post_url(FLOW_EDIT_MODAL, 
                &modal_edit_post_url(InputEditPostRouteTag::new(id), &self.form_name),
            ),
            (FlowSide::Output, Some(id)) => form_hx_post_url(FLOW_EDIT_MODAL, 
                &modal_edit_post_url(OutputEditPostRouteTag::new(id), &self.form_name),
            ),
            (FlowSide::Input, None) => {
                form_hx_post_url(FLOW_CREATE_MODAL, &modal_create_post_url(
                    InputCreatePostRouteTag::new(self.step_id),
                    &self.form_name,
                    "",
                ))
            }
            (FlowSide::Output, None) => {
                form_hx_post_url(FLOW_CREATE_MODAL, &modal_create_post_url(
                    OutputCreatePostRouteTag::new(self.step_id),
                    &self.form_name,
                    "",
                ))
            }
        };
        let delete = self.existing_id.map(|id| {
            let (href, name) = match self.side {
                FlowSide::Input => (InputDeleteGetRouteTag::new(id).url(), "wo.InputDeleteForm"),
                FlowSide::Output => (
                    OutputDeleteGetRouteTag::new(id).url(),
                    "wo.OutputDeleteForm",
                ),
            };
            button_modal_form(ButtonModalForm {
                label: "Delete",
                icon_name: Some("trash"),
                name,
                href: &href,
                form_post_url: &href,
                modal_uid: FLOW_DELETE_MODAL.id(),
                classes: "btn-error",
                ..Default::default()
            })
        });
        let body = form(
            &CsrfToken::current(),
            FormOpts {
                title,
                attrs,
                form_error: Some(self.error.as_str()).filter(|err| !err.is_empty()),
                inputs: FlowForm::render_inputs(&ctx),
                actions: html! {
                    (submit("Save"))
                    @if let Some(button) = delete {
                        (button)
                    }
                },
                ..Default::default()
            },
        );
        match self.existing_id {
            Some(_) => modal_keyed(FLOW_EDIT_MODAL, "", body),
            None => modal_keyed(FLOW_CREATE_MODAL, "", body),
        }
    }
}

pub struct CompleteLineView {
    pub part_id: i64,
    pub name: String,
    pub qty_value: String,
    pub qty_unit: String,
}

pub struct CompleteModalPage {
    pub step_id: i64,
    pub step_name: String,
    pub form_name: String,
    pub inputs: Vec<CompleteLineView>,
    pub outputs: Vec<CompleteLineView>,
    pub error: String,
}

impl RenderTemplate for CompleteModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        modal_keyed(COMPLETE_MODAL, 
            "",
            form(
                &CsrfToken::current(),
                FormOpts {
                    title: "Record batch",
                    subtitle: &self.step_name,
                    attrs: form_hx_post_url(COMPLETE_MODAL, &modal_edit_post_url(
                        CompletePostRouteTag::new(self.step_id),
                        &self.form_name,
                    )),
                    form_error: Some(self.error.as_str()).filter(|err| !err.is_empty()),
                    inputs: html! {
                        h4 class="font-medium" { "Inputs" }
                        @for line in &self.inputs {
                            (qty_fields("in", line))
                        }
                        h4 class="font-medium mt-2" { "Outputs" }
                        @for line in &self.outputs {
                            (qty_fields("out", line))
                        }
                    },
                    actions: submit("Record batch"),
                    ..Default::default()
                },
            ),
        )
    }
}

fn qty_fields(prefix: &str, line: &CompleteLineView) -> Markup {
    let value_name = format!("{prefix}_value_{}", line.part_id);
    let unit_name = format!("{prefix}_unit_{}", line.part_id);
    html! {
        div class="grid grid-cols-2 gap-2" {
            label class="form-control" {
                span class="label-text" { (format!("{} quantity", line.name)) }
                input class="input input-bordered" type="text" name=(value_name) value=(line.qty_value) {}
            }
            label class="form-control" {
                span class="label-text" { "Unit" }
                input class="input input-bordered" type="text" name=(unit_name) value=(line.qty_unit) {}
            }
        }
    }
}

pub struct ConfirmDeleteModalPage {
    pub modal_uid: &'static str,
    pub title: String,
    pub message: String,
    pub post_url: String,
    pub error: String,
}

impl RenderTemplate for ConfirmDeleteModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        modal(Modal {
            uid: self.modal_uid,
            children: delete_confirmation(DeleteConfirmation {
                title: &self.title,
                message: &self.message,
                attrs: form_hx_post_selector(&self.post_url, &format!("#{}", self.modal_uid)),
                global_error: Some(self.error.as_str()).filter(|err| !err.is_empty()),
                ..Default::default()
            }),
            ..Default::default()
        })
    }
}

pub fn detail_redirect(id: i64) -> String {
    detail_url(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::work_order;

    #[test]
    fn work_order_form_edits_the_plan_and_detail_has_no_add_buttons() {
        let page = WorkOrderCreateModalPage {
            form_name: String::new(),
            refresh_table: String::new(),
            title: String::new(),
            customer_id: String::new(),
            customer_display: String::new(),
            folder_vnode_id: String::new(),
            folder_display: String::new(),
            parts_json: "[]".into(),
            presupplied_json: "[]".into(),
            steps_json: "[]".into(),
            error: String::new(),
        };
        let html = page.render(&Default::default()).into_string();
        assert!(html.contains("node-select"), "vnode picker");
        assert!(
            html.contains("Select a file or folder"),
            "vnode placeholder"
        );
        assert!(
            html.contains(">Parts<") || html.contains(">Parts"),
            "parts label"
        );
        assert!(html.contains("Presupplied parts"), "presupplied label");
        assert!(
            html.contains(">Steps<") || html.contains(">Steps"),
            "steps label"
        );
        assert!(
            html.contains("name=\"PresuppliedLinesJson\""),
            "presupplied lines"
        );
        assert!(html.contains("name=\"StepsLinesJson\""), "step lines");
        assert!(html.contains("Add line"), "line editor");
        assert!(html.contains("Inputs per batch"), "step inputs");
        assert!(
            html.contains("woPlanParts"),
            "presupplied parts follow parts"
        );

        let detail = WorkOrderDetailPage {
            plan: Plan {
                order: work_order::Model {
                    id: 1,
                    created_at: None,
                    updated_at: None,
                    title: "Bracket".into(),
                    customer_id: 1,
                    folder_vnode_id: 1,
                },
                parts: vec![part::Model {
                    id: 1,
                    created_at: None,
                    updated_at: None,
                    work_order_id: 1,
                    name: "Plate".into(),
                    measure_kind: "quantity".into(),
                    measure_unit: "Number".into(),
                }],
                presupplied: Vec::new(),
                steps: vec![step::Model {
                    id: 1,
                    created_at: None,
                    updated_at: None,
                    work_order_id: 1,
                    name: "Cut".into(),
                    planned_batches: 2,
                }],
                inputs: Vec::new(),
                outputs: Vec::new(),
                completions: Vec::new(),
                completion_inputs: Vec::new(),
                completion_outputs: Vec::new(),
            },
            customer: "Acme".into(),
            customer_href: "/customers/1/".into(),
            files_href: "/dashboard/filesystem/1/".into(),
            files_label: "Drawings".into(),
            layout: Some(PlanLayout {
                roles: vec![(1, PartRole::Raw)],
                part_ranks: vec![graph::RankedNode { id: 1, rank: 0 }],
                step_ranks: Vec::new(),
                coverage: Vec::new(),
            }),
            layout_error: String::new(),
        };
        let html = detail.render(&Default::default()).into_string();
        assert!(html.contains("Drawings"), "file label");
        assert!(html.contains("Plate"), "part row");
        assert!(html.contains("Record batch"), "completion stays");
        assert!(!html.contains("Add part"), "no add part button");
        assert!(!html.contains("Add supply"), "no add supply button");
        assert!(!html.contains("Add step"), "no add step button");
        assert!(
            html.contains("width=\"128\""),
            "graph stays at its own size"
        );
        assert!(html.contains("max-h-52"), "graph box is capped");
    }
}
