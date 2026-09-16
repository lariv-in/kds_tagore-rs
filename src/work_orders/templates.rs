use frunk::Generic;
use maud::PreEscaped;
use lariv_rs::{
    components::{
        attrs::escape_attr, CodeEditorInput, button_modal_form, button_submit, code_editor_input,
        container_column, container_row, data_table_list_refresh, delete_confirmation, detail,
        detail_header, field_text, form, label, layout_main, layout_sidebar, modal, modal_keyed,
        row_attr_navigate_route, row_attr_select, row_attr_select_extra, shell_scaffold,
        table_create_button, ButtonModalForm, ButtonSubmit, DeleteConfirmation, DetailHeader,
        FieldText, FormOpts, HTMX_SWAP_BODY_MODAL, HTMX_TARGET_BODY_MODAL, LayoutMain,
        LayoutSidebar, ShellChrome, ShellScaffold, SlotCapability, SlotRegistrar, SwapKey,
        TableColumnHeader, TableRow,
    },
    http::ProvideRequestCaps,
    picker::{picker_create_button, RenderPickerSelect},
    template::{RenderAppPane, RenderTemplate, TemplateCapability, TemplateOf, TemplateRegistrar},
};
use maud::{Markup, html};

use crate::machinery_schedule::logic::format_job_duration;

use lariv_rs::html_form::{FormCtx, HtmlForm};

use rust_decimal::Decimal;

use super::crumbs::*;
use super::entities::{
    component, draft_work_order_machine_line, draft_work_order_material_line, machine, material,
    material_rate, proforma_invoice, proforma_invoice_machine_line, proforma_invoice_material_line,
    shape, work_order,
};
use super::forms::{
    ComponentForm, ComponentFormField, DraftWorkOrderForm, DraftWorkOrderFormField,
    DraftWorkOrderLineForm, DraftWorkOrderLineFormField, DraftWorkOrderMachineLineForm,
    DraftWorkOrderMachineLineFormField, InvoiceForm, InvoiceFormField, ShapeForm, ShapeFormField,
};
use super::keys::*;
use super::routes::*;

fn order_number_display(s: &str) -> &str {
    if s.trim().is_empty() {
        "—"
    } else {
        s
    }
}

lariv_rs::define_register_items! {
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
        WorkOrderSelectPageIdx: WorkOrderSelectPageTag => WorkOrderSelectPage,

        ComponentListPageIdx: ComponentListPageTag => ComponentListPage,
        ComponentDetailPageIdx: ComponentDetailPageTag => ComponentDetailPage,
        ComponentCreateModalPageIdx: ComponentCreateModalPageTag => ComponentCreateModalPage,
        ComponentEditModalPageIdx: ComponentEditModalPageTag => ComponentEditModalPage,
        ComponentSelectPageIdx: ComponentSelectPageTag => ComponentSelectPage,

        ShapeListPageIdx: ShapeListPageTag => ShapeListPage,
        ShapeDetailPageIdx: ShapeDetailPageTag => ShapeDetailPage,
        ShapeCreateModalPageIdx: ShapeCreateModalPageTag => ShapeCreateModalPage,
        ShapeEditModalPageIdx: ShapeEditModalPageTag => ShapeEditModalPage,
        ShapeSelectPageIdx: ShapeSelectPageTag => ShapeSelectPage,

        MaterialListPageIdx: MaterialListPageTag => MaterialListPage,
        MaterialDetailPageIdx: MaterialDetailPageTag => MaterialDetailPage,
        MaterialCreateModalPageIdx: MaterialCreateModalPageTag => MaterialCreateModalPage,
        MaterialEditModalPageIdx: MaterialEditModalPageTag => MaterialEditModalPage,
        MaterialSelectPageIdx: MaterialSelectPageTag => MaterialSelectPage,

        MaterialRateListPageIdx: MaterialRateListPageTag => MaterialRateListPage,
        MaterialRateCreateModalPageIdx: MaterialRateCreateModalPageTag => MaterialRateCreateModalPage,

        MachineListPageIdx: MachineListPageTag => MachineListPage,
        MachineDetailPageIdx: MachineDetailPageTag => MachineDetailPage,
        MachineCreateModalPageIdx: MachineCreateModalPageTag => MachineCreateModalPage,
        MachineEditModalPageIdx: MachineEditModalPageTag => MachineEditModalPage,

        InvoiceListPageIdx: InvoiceListPageTag => InvoiceListPage,
        InvoiceDetailPageIdx: InvoiceDetailPageTag => InvoiceDetailPage,
        InvoiceCreateModalPageIdx: InvoiceCreateModalPageTag => InvoiceCreateModalPage,
        InvoiceEditModalPageIdx: InvoiceEditModalPageTag => InvoiceEditModalPage,

        ConfirmDeleteModalPageIdx: ConfirmDeleteModalPageTag => ConfirmDeleteModalPage,
        WorkOrderLineEditModalPageIdx: WorkOrderLineEditModalPageTag => WorkOrderLineEditModalPage,
        WorkOrderMachineLineEditModalPageIdx: WorkOrderMachineLineEditModalPageTag => WorkOrderMachineLineEditModalPage,
        MachineSelectPageIdx: MachineSelectPageTag => MachineSelectPage,
        WorkOrdersPreferencesPageIdx: WorkOrdersPreferencesPageTag => WorkOrdersPreferencesPage,
    ]
}

lariv_rs::define_register_items! {
    plugin: super::WorkOrdersTag;
    capability: SlotCapability;
    trait: SlotRegistrar;
    method: register_slots;
    bounds: [];
    items: [];
    hook: SlotsHook;
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
) -> lariv_rs::components::AppLayoutHtml {
    layout_sidebar(LayoutSidebar {
        sidebar,
        breadcrumbs: crumbs,
        content: body,
    })
}

fn scaffold_main(crumbs: Markup, body: Markup) -> lariv_rs::components::MainContentHtml {
    layout_main(LayoutMain {
        breadcrumbs: crumbs,
        content: body,
    })
}

// ==========================================
// 1. WORK ORDERS
// ==========================================

#[derive(Clone, Generic)]
pub struct WorkOrderListPage {
    pub orders: Vec<(work_order::Model, usize, Decimal)>,
    pub path_and_query: String,
}

impl WorkOrderListPage {
    pub fn render_table(&self) -> Markup {
        let headers = [
            TableColumnHeader { key: "Id", label: "Id", sort_url: None, push_url: false },
            TableColumnHeader { key: "OrderNumber", label: "Order #", sort_url: None, push_url: false },
            TableColumnHeader { key: "CustomerId", label: "Customer ID", sort_url: None, push_url: false },
            TableColumnHeader { key: "Lines", label: "Lines", sort_url: None, push_url: false },
            TableColumnHeader { key: "TotalAmount", label: "Total Cost", sort_url: None, push_url: false },
        ];

        let id_labels: Vec<String> = self.orders.iter().map(|(o, _, _)| o.id.to_string()).collect();
        let cust_labels: Vec<String> = self.orders.iter().map(|(o, _, _)| o.customer_id.to_string()).collect();
        let lines_labels: Vec<String> = self.orders.iter().map(|(_, count, _)| count.to_string()).collect();
        let total_labels: Vec<String> = self.orders.iter().map(|(_, _, total)| format!("₹ {:.2}", total)).collect();

        let rows: Vec<TableRow> = self.orders.iter().enumerate().map(|(i, (o, _, _))| TableRow {
            attrs: row_attr_navigate_route(WorkOrderDetailRouteTag::new(o.id)),
            cells: vec![
                field_text(FieldText { value: &id_labels[i], classes: "" }),
                field_text(FieldText { value: order_number_display(&o.order_number), classes: "font-semibold" }),
                field_text(FieldText { value: &cust_labels[i], classes: "" }),
                field_text(FieldText { value: &lines_labels[i], classes: "" }),
                field_text(FieldText { value: &total_labels[i], classes: "font-mono font-semibold" }),
            ],
        }).collect();

        let actions = html! {
            (table_create_button::<WorkOrderTableKey, WorkOrderCreateModalKey>(
                Some("plus"),
                "btn-square btn-outline btn-sm",
            ))
        };

        data_table_list_refresh::<WorkOrderTableKey>(
            "Draft Work Orders",
            actions,
            &headers,
            &rows,
            html! {},
            &self.path_and_query,
        )
    }
}

impl RenderAppPane for WorkOrderListPage {
    fn render_pane(&self) -> lariv_rs::components::AppLayoutHtml {
        scaffold_pane(wo_menu("orders"), work_orders_list_crumbs(), self.render_table())
    }
    fn render_main(&self) -> lariv_rs::components::MainContentHtml {
        scaffold_main(work_orders_list_crumbs(), self.render_table())
    }
}

impl RenderTemplate for WorkOrderListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold("Draft Work Orders", chrome, wo_menu("orders"), work_orders_list_crumbs(), self.render_table())
    }
}

#[derive(Clone, Generic)]
pub struct WorkOrderDetailPage {
    pub order: work_order::Model,
    pub lines: Vec<(draft_work_order_material_line::Model, String)>,
    pub machine_lines: Vec<(draft_work_order_machine_line::Model, String)>,
    pub customer_name: Option<String>,
    pub total_amount: Decimal,
}

impl WorkOrderDetailPage {
    fn body(&self) -> Markup {
        let edit_url = WorkOrderEditGetRouteTag::new(self.order.id).url();
        let pdf_url = WorkOrderPdfRouteTag::new(self.order.id).url();
        let actions = html! {
            (button_modal_form(ButtonModalForm {
                label: "Edit",
                icon_name: Some("pencil"),
                name: "wo.WorkOrderEditForm",
                href: &edit_url,
                form_post_url: &edit_url,
                modal_uid: WorkOrderEditModalKey::ID,
                classes: "btn-outline btn-sm",
                ..Default::default()
            }))
            a class="btn btn-outline btn-sm" href=(pdf_url) target="_blank" rel="noopener" {
                "Download PDF"
            }
        };

        let cust_label = match &self.customer_name {
            Some(name) => format!("{} (#{})", name, self.order.customer_id),
            None => format!("#{}", self.order.customer_id),
        };
        let lines_count_str = self.lines.len().to_string();
        let machine_lines_count_str = self.machine_lines.len().to_string();
        let total_str = format!("₹ {:.2}", self.total_amount);
        let title_str = if self.order.order_number.trim().is_empty() {
            "Draft Work Order".to_string()
        } else {
            format!("Draft Work Order {}", self.order.order_number)
        };

        html! {
            (detail(html! {
                (container_column("", html! {
                    (detail_header(DetailHeader {
                        title: &title_str,
                        actions,
                    }))
                    (container_row("gap-6", html! {
                        (label("Customer", field_text(FieldText { value: &cust_label, classes: "" })))
                        (label("Material Lines", field_text(FieldText { value: &lines_count_str, classes: "" })))
                        (label("Machine Lines", field_text(FieldText { value: &machine_lines_count_str, classes: "" })))
                        (label("Total Cost", field_text(FieldText { value: &total_str, classes: "font-mono font-bold text-primary" })))
                    }))

                    div class="mt-8" {
                        div class="mb-3" {
                            h4 class="font-bold text-lg" { "Draft Work Order Material Lines" }
                        }

                        @if self.lines.is_empty() {
                            div class="p-8 text-center text-sm text-base-content/60 bg-base-200/50 rounded-lg border border-base-200" {
                                "No line items added yet."
                            }
                        } @else {
                            div class="overflow-x-auto border border-base-200 rounded-lg" {
                                table class="table table-zebra w-full text-sm" {
                                    thead {
                                        tr {
                                            th class="w-12" { "#" }
                                            th { "Component" }
                                            th { "Dimensions / Variables" }
                                            th class="text-right" { "Weight (kg)" }
                                            th class="text-right" { "Rate (₹/kg)" }
                                            th class="text-right" { "Quantity" }
                                            th class="text-right" { "Final Cost (₹)" }
                                        }
                                    }
                                    tbody {
                                        @for (idx, (l, comp_name)) in self.lines.iter().enumerate() {
                                            tr {
                                                td class="opacity-60" { (idx + 1) }
                                                td class="font-semibold" { (comp_name) }
                                                td class="font-mono text-xs" { (l.format_variables_display()) }
                                                td class="text-right font-mono" { (format!("{:.3}", l.unit_weight)) }
                                                td class="text-right font-mono" { (format!("₹ {:.2}", l.material_rate)) }
                                                td class="text-right font-mono" { (format!("{:.2}", l.quantity)) }
                                                td class="text-right font-mono font-bold text-primary" { (format!("₹ {:.2}", l.final_cost)) }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    div class="mt-8" {
                        div class="mb-3" {
                            h4 class="font-bold text-lg" { "Draft Work Order Machine Lines" }
                        }

                        @if self.machine_lines.is_empty() {
                            div class="p-8 text-center text-sm text-base-content/60 bg-base-200/50 rounded-lg border border-base-200" {
                                "No machine lines added yet."
                            }
                        } @else {
                            div class="overflow-x-auto border border-base-200 rounded-lg" {
                                table class="table table-zebra w-full text-sm" {
                                    thead {
                                        tr {
                                            th class="w-12" { "#" }
                                            th { "Machine" }
                                            th class="text-right" { "Rate (₹/hr)" }
                                            th class="text-right" { "Duration" }
                                            th class="text-right" { "Total (₹)" }
                                        }
                                    }
                                    tbody {
                                        @for (idx, (l, machine_name)) in self.machine_lines.iter().enumerate() {
                                            tr {
                                                td class="opacity-60" { (idx + 1) }
                                                td class="font-semibold" { (machine_name) }
                                                td class="text-right font-mono" { (format!("₹ {:.2}", l.rate_decimal)) }
                                                td class="text-right font-mono" { (format_job_duration(l.time_used)) }
                                                td class="text-right font-mono font-bold text-primary" { (format!("₹ {:.2}", l.line_total())) }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    div class="mt-4 p-4 bg-base-200/60 rounded-lg flex justify-between items-center border border-base-200" {
                        span class="font-semibold" { "Final Grand Total" }
                        span class="text-xl font-extrabold text-primary font-mono" { (format!("₹ {:.2}", self.total_amount)) }
                    }
                }))
            }))
        }
    }
}

impl RenderAppPane for WorkOrderDetailPage {
    fn render_pane(&self) -> lariv_rs::components::AppLayoutHtml {
        scaffold_pane(wo_menu("orders"), work_order_crumbs(order_number_display(&self.order.order_number), self.order.id), self.body())
    }
    fn render_main(&self) -> lariv_rs::components::MainContentHtml {
        scaffold_main(work_order_crumbs(order_number_display(&self.order.order_number), self.order.id), self.body())
    }
}

impl RenderTemplate for WorkOrderDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        let page_title = if self.order.order_number.trim().is_empty() {
            "Draft Work Order — Draft Work Orders".to_string()
        } else {
            format!("Order {} — Draft Work Orders", self.order.order_number)
        };
        app_scaffold(&page_title, chrome, wo_menu("orders"), work_order_crumbs(order_number_display(&self.order.order_number), self.order.id), self.body())
    }
}

#[derive(Clone, Generic, Default)]
pub struct WorkOrderCreateModalPage {
    pub form_name: String,
    pub order_number: String,
    pub customer_id: Option<i64>,
    pub customer_name: String,
    pub items_json: String,
    pub components_json: String,
    pub machine_lines_json: String,
    pub machines_json: String,
    pub error: String,
}

impl RenderTemplate for WorkOrderCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let cust_id_str = self.customer_id.map(|id| id.to_string()).unwrap_or_default();
        let items_val = if self.items_json.is_empty() { "[]" } else { &self.items_json };
        let machine_lines_val = if self.machine_lines_json.is_empty() { "[]" } else { &self.machine_lines_json };
        let ctx = FormCtx::form::<DraftWorkOrderForm>()
            .value(DraftWorkOrderFormField::OrderNumber, &self.order_number)
            .value(DraftWorkOrderFormField::CustomerId, &cust_id_str)
            .display(DraftWorkOrderFormField::CustomerId, &self.customer_name)
            .value(DraftWorkOrderFormField::Items, items_val)
            .display(DraftWorkOrderFormField::Items, &self.components_json)
            .value(DraftWorkOrderFormField::MachineLines, machine_lines_val)
            .display(DraftWorkOrderFormField::MachineLines, &self.machines_json);

        let modal_classes = format!("!max-w-6xl !w-11/12 {}", self.form_name);
        modal_keyed::<DraftWorkOrderCreateModalKey>(
            &modal_classes,
            html! {
                h3 class="font-bold text-lg mb-4" { "New Draft Work Order" }
                @if !self.error.is_empty() {
                    div class="alert alert-error text-sm mb-4 shadow-sm" {
                        span { (self.error) }
                    }
                }
                (form(FormOpts {
                    attrs: lariv_rs::components::swap::form_hx_post_url::<DraftWorkOrderCreateModalKey>(
                        &WorkOrderCreatePostRouteTag.url(),
                    ),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: html! {
                        (DraftWorkOrderForm::render_inputs(&ctx))
                    },
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Create Draft Work Order", ..Default::default() }))
                    },
                    ..Default::default()
                }))
            },
        )
    }
}

#[derive(Clone, Generic)]
pub struct WorkOrderEditModalPage {
    pub id: i64,
    pub form_name: String,
    pub order_number: String,
    pub customer_id: i64,
    pub customer_name: String,
    pub items_json: String,
    pub components_json: String,
    pub machine_lines_json: String,
    pub machines_json: String,
    pub error: String,
}

impl RenderTemplate for WorkOrderEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let cust_id_str = self.customer_id.to_string();
        let items_val = if self.items_json.is_empty() { "[]" } else { &self.items_json };
        let machine_lines_val = if self.machine_lines_json.is_empty() { "[]" } else { &self.machine_lines_json };
        let ctx = FormCtx::form::<DraftWorkOrderForm>()
            .value(DraftWorkOrderFormField::OrderNumber, &self.order_number)
            .value(DraftWorkOrderFormField::CustomerId, &cust_id_str)
            .display(DraftWorkOrderFormField::CustomerId, &self.customer_name)
            .value(DraftWorkOrderFormField::Items, items_val)
            .display(DraftWorkOrderFormField::Items, &self.components_json)
            .value(DraftWorkOrderFormField::MachineLines, machine_lines_val)
            .display(DraftWorkOrderFormField::MachineLines, &self.machines_json);
        let delete_url = WorkOrderDeleteGetRouteTag::new(self.id).url();

        let modal_classes = format!("!max-w-6xl !w-11/12 {}", self.form_name);
        modal_keyed::<DraftWorkOrderEditModalKey>(
            &modal_classes,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit Draft Work Order" }
                @if !self.error.is_empty() {
                    div class="alert alert-error text-sm mb-4 shadow-sm" {
                        span { (self.error) }
                    }
                }
                (form(FormOpts {
                    attrs: lariv_rs::components::swap::form_hx_post_url::<DraftWorkOrderEditModalKey>(
                        &WorkOrderEditPostRouteTag::new(self.id).url(),
                    ),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: html! {
                        (DraftWorkOrderForm::render_inputs(&ctx))
                    },
                    actions: html! {
                        (button_modal_form(ButtonModalForm {
                            label: "Delete",
                            icon_name: Some("trash"),
                            name: "wo.WorkOrderDeleteForm",
                            href: &delete_url,
                            form_post_url: &delete_url,
                            modal_uid: WorkOrderDeleteModalKey::ID,
                            classes: "btn-error btn-sm",
                            ..Default::default()
                        }))
                        (button_submit(ButtonSubmit { label: "Save Changes", ..Default::default() }))
                    },
                    ..Default::default()
                }))
            },
        )
    }
}

#[derive(Clone, Generic)]
pub struct WorkOrderLineEditModalPage {
    pub id: i64,
    pub draft_work_order_id: i64,
    pub draft_work_order_label: String,
    pub form_name: String,
    pub component_id: i64,
    pub component_label: String,
    pub variables: String,
    pub quantity: String,
    pub extra_data: String,
    pub error: String,
}

impl RenderTemplate for WorkOrderLineEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let wo_id_str = self.draft_work_order_id.to_string();
        let comp_id_str = self.component_id.to_string();
        let qty_str = if self.quantity.is_empty() { "1" } else { &self.quantity };
        let vars_str = if self.variables.is_empty() { "{}" } else { &self.variables };
        let ctx = FormCtx::form::<DraftWorkOrderLineForm>()
            .value(DraftWorkOrderLineFormField::DraftWorkOrderId, &wo_id_str)
            .display(DraftWorkOrderLineFormField::DraftWorkOrderId, &self.draft_work_order_label)
            .value(DraftWorkOrderLineFormField::ComponentId, &comp_id_str)
            .display(DraftWorkOrderLineFormField::ComponentId, &self.component_label)
            .value(DraftWorkOrderLineFormField::Variables, vars_str)
            .value(DraftWorkOrderLineFormField::Quantity, qty_str)
            .value(DraftWorkOrderLineFormField::ExtraData, &self.extra_data);

        modal_keyed::<DraftWorkOrderLineEditModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit Draft Work Order Material Line" }
                @if !self.error.is_empty() {
                    div class="alert alert-error text-sm mb-4 shadow-sm" {
                        span { (self.error) }
                    }
                }
                (form(FormOpts {
                    attrs: lariv_rs::components::swap::form_hx_post_url::<DraftWorkOrderLineEditModalKey>(
                        &WorkOrderLineEditPostRouteTag::new(self.id).url(),
                    ),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: html! {
                        (DraftWorkOrderLineForm::render_inputs(&ctx))
                    },
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Save Changes", ..Default::default() }))
                    },
                    ..Default::default()
                }))
            },
        )
    }
}

#[derive(Clone, Generic)]
pub struct WorkOrderMachineLineEditModalPage {
    pub id: i64,
    pub form_name: String,
    pub draft_work_order_id: i64,
    pub draft_work_order_label: String,
    pub machine_id: i64,
    pub machine_label: String,
    pub rate: String,
    pub duration: String,
    pub error: String,
}

impl RenderTemplate for WorkOrderMachineLineEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let wo_id_str = self.draft_work_order_id.to_string();
        let machine_id_str = self.machine_id.to_string();
        let ctx = FormCtx::form::<DraftWorkOrderMachineLineForm>()
            .value(DraftWorkOrderMachineLineFormField::DraftWorkOrderId, &wo_id_str)
            .display(DraftWorkOrderMachineLineFormField::DraftWorkOrderId, &self.draft_work_order_label)
            .value(DraftWorkOrderMachineLineFormField::MachineId, &machine_id_str)
            .display(DraftWorkOrderMachineLineFormField::MachineId, &self.machine_label)
            .value(DraftWorkOrderMachineLineFormField::Rate, &self.rate)
            .value(DraftWorkOrderMachineLineFormField::Duration, &self.duration);

        let rate_input = "input[name=Rate], input[name=rate], input[name=RATE]";
        let alpine_wrap = format!(
            r#"<div x-data="{{ onMachineSelect(d) {{ if (d && d.rate !== undefined) {{ try {{ const i = this.$el.querySelector('{rate_input}'); if (i) i.value = String(d.rate) }} catch (e) {{}} }} }} }}" @fk-select.window="onMachineSelect($event.detail)">"#);

        modal_keyed::<DraftWorkOrderMachineLineEditModalKey>(
            &self.form_name,
            html! {
                (PreEscaped(alpine_wrap))
                    h3 class="font-bold text-lg mb-4" { "Edit Draft Work Order Machine Line" }
                    @if !self.error.is_empty() {
                        div class="alert alert-error text-sm mb-4 shadow-sm" {
                            span { (self.error) }
                        }
                    }
                    (form(FormOpts {
                        attrs: lariv_rs::components::swap::form_hx_post_url::<DraftWorkOrderMachineLineEditModalKey>(
                            &WorkOrderMachineLineEditPostRouteTag::new(self.id).url(),
                        ),
                        form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                        inputs: html! {
                            (DraftWorkOrderMachineLineForm::render_inputs(&ctx))
                        },
                        actions: html! {
                            (button_submit(ButtonSubmit { label: "Save Changes", ..Default::default() }))
                        },
                        ..Default::default()
                    }))
                    (PreEscaped("</div>"))
                },
        )
    }
}

#[derive(Clone, Generic, Default)]
pub struct MachineSelectPage {
    pub machines: Vec<(machine::Model, String)>,
    pub target_input: String,
    pub path_and_query: String,
}

impl RenderPickerSelect<MachineSelectTableKey, MachineSelectModalKey> for MachineSelectPage {
    fn render_table(&self) -> Markup {
        let target = if self.target_input.is_empty() {
            "machine_id"
        } else {
            self.target_input.as_str()
        };
        let headers = [
            TableColumnHeader { key: "Name", label: "Machine", sort_url: None, push_url: false },
            TableColumnHeader { key: "Rate", label: "Rate (₹/hr)", sort_url: None, push_url: false },
        ];
        let rows: Vec<TableRow> = self.machines.iter().map(|(m, rate_str)| {
            let rate_num = m.rate_decimal.to_string();
            TableRow {
                attrs: row_attr_select_extra(target, &m.id.to_string(), &m.name, &[("rate", rate_num.as_str())]),
                cells: vec![
                    field_text(FieldText { value: &m.name, classes: "font-semibold" }),
                    field_text(FieldText { value: rate_str, classes: "font-mono" }),
                ],
            }
        }).collect();

        let actions = html! {};

        data_table_list_refresh::<MachineSelectTableKey>(
            "Select Machine",
            actions,
            &headers,
            &rows,
            html! {},
            &self.path_and_query,
        )
    }
}

impl RenderTemplate for MachineSelectPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        self.render_modal().into_inner()
    }
}

#[derive(Clone, Generic)]
pub struct WorkOrderSelectPage {
    pub orders: Vec<work_order::Model>,
    pub target_input: String,
    pub path_and_query: String,
}

impl RenderPickerSelect<WorkOrderSelectTableKey, WorkOrderSelectModalKey> for WorkOrderSelectPage {
    fn render_table(&self) -> Markup {
        let target = if self.target_input.is_empty() {
            "work_order_id"
        } else {
            self.target_input.as_str()
        };
        let headers = [
            TableColumnHeader { key: "OrderNumber", label: "Order #", sort_url: None, push_url: false },
            TableColumnHeader { key: "CustomerId", label: "Customer ID", sort_url: None, push_url: false },
        ];
        let rows: Vec<TableRow> = self.orders.iter().map(|o| {
            TableRow {
                attrs: row_attr_select(target, &o.id.to_string(), order_number_display(&o.order_number)),
                cells: vec![
                    field_text(FieldText { value: order_number_display(&o.order_number), classes: "font-semibold" }),
                    field_text(FieldText { value: &o.customer_id.to_string(), classes: "font-mono" }),
                ],
            }
        }).collect();

        let actions = html! {};

        data_table_list_refresh::<WorkOrderSelectTableKey>(
            "Select Work Order",
            actions,
            &headers,
            &rows,
            html! {},
            &self.path_and_query,
        )
    }
}

impl RenderTemplate for WorkOrderSelectPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        self.render_modal().into_inner()
    }
}

pub type DraftWorkOrderListPage = WorkOrderListPage;
pub type DraftWorkOrderDetailPage = WorkOrderDetailPage;
pub type DraftWorkOrderCreateModalPage = WorkOrderCreateModalPage;
pub type DraftWorkOrderEditModalPage = WorkOrderEditModalPage;
pub type DraftWorkOrderSelectPage = WorkOrderSelectPage;
pub type DraftWorkOrderLineEditModalPage = WorkOrderLineEditModalPage;
pub type DraftWorkOrderMaterialLineEditModalPage = WorkOrderLineEditModalPage;
pub type WorkOrderMaterialLineEditModalPage = WorkOrderLineEditModalPage;
pub type DraftWorkOrderMachineLineEditModalPage = WorkOrderMachineLineEditModalPage;

#[derive(Clone, Generic)]
pub struct ComponentSelectPage {
    pub components: Vec<(component::Model, String, String)>,
    pub target_input: String,
    pub path_and_query: String,
}

impl RenderPickerSelect<ComponentSelectTableKey, ComponentSelectModalKey> for ComponentSelectPage {
    fn render_table(&self) -> Markup {
        let target = if self.target_input.is_empty() {
            "component_id"
        } else {
            self.target_input.as_str()
        };
        let headers = [
            TableColumnHeader { key: "Name", label: "Name", sort_url: None, push_url: false },
            TableColumnHeader { key: "Shape", label: "Shape", sort_url: None, push_url: false },
            TableColumnHeader { key: "Material", label: "Material", sort_url: None, push_url: false },
        ];
        let rows: Vec<TableRow> = self.components.iter().map(|(c, s_name, m_name)| {
            TableRow {
                attrs: row_attr_select(target, &c.id.to_string(), &c.name),
                cells: vec![
                    field_text(FieldText { value: &c.name, classes: "font-semibold" }),
                    field_text(FieldText { value: s_name, classes: "" }),
                    field_text(FieldText { value: m_name, classes: "" }),
                ],
            }
        }).collect();

        let actions = html! {};

        data_table_list_refresh::<ComponentSelectTableKey>(
            "Select Component",
            actions,
            &headers,
            &rows,
            html! {},
            &self.path_and_query,
        )
    }
}

impl RenderTemplate for ComponentSelectPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        self.render_modal().into_inner()
    }
}

// ==========================================
// 2. COMPONENTS
// ==========================================

#[derive(Clone, Generic)]
pub struct ComponentListPage {
    pub components: Vec<(component::Model, Option<shape::Model>, Option<material::Model>)>,
    pub path_and_query: String,
}

impl ComponentListPage {
    pub fn render_table(&self) -> Markup {
        let headers = [
            TableColumnHeader { key: "Id", label: "Id", sort_url: None, push_url: false },
            TableColumnHeader { key: "Name", label: "Name", sort_url: None, push_url: false },
            TableColumnHeader { key: "Shape", label: "Shape", sort_url: None, push_url: false },
            TableColumnHeader { key: "Material", label: "Material", sort_url: None, push_url: false },
        ];

        let id_labels: Vec<String> = self.components.iter().map(|(c, _, _)| c.id.to_string()).collect();
        let shape_labels: Vec<String> = self.components.iter().map(|(_, s, _)| s.as_ref().map(|x| x.name.clone()).unwrap_or_else(|| "-".into())).collect();
        let mat_labels: Vec<String> = self.components.iter().map(|(_, _, m)| m.as_ref().map(|x| x.name.clone()).unwrap_or_else(|| "-".into())).collect();

        let rows: Vec<TableRow> = self.components.iter().enumerate().map(|(i, (c, _, _))| TableRow {
            attrs: row_attr_navigate_route(ComponentDetailRouteTag::new(c.id)),
            cells: vec![
                field_text(FieldText { value: &id_labels[i], classes: "" }),
                field_text(FieldText { value: &c.name, classes: "font-semibold" }),
                field_text(FieldText { value: &shape_labels[i], classes: "" }),
                field_text(FieldText { value: &mat_labels[i], classes: "" }),
            ],
        }).collect();

        let actions = html! {
            (table_create_button::<ComponentTableKey, ComponentCreateModalKey>(
                Some("plus"),
                "btn-square btn-outline btn-sm",
            ))
        };

        data_table_list_refresh::<ComponentTableKey>(
            "Components",
            actions,
            &headers,
            &rows,
            html! {},
            &self.path_and_query,
        )
    }
}

impl RenderAppPane for ComponentListPage {
    fn render_pane(&self) -> lariv_rs::components::AppLayoutHtml {
        scaffold_pane(wo_menu("components"), components_list_crumbs(), self.render_table())
    }
    fn render_main(&self) -> lariv_rs::components::MainContentHtml {
        scaffold_main(components_list_crumbs(), self.render_table())
    }
}

impl RenderTemplate for ComponentListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold("Components — Work Orders", chrome, wo_menu("components"), components_list_crumbs(), self.render_table())
    }
}

#[derive(Clone, Generic)]
pub struct ComponentDetailPage {
    pub component: component::Model,
    pub shape: Option<shape::Model>,
    pub material: Option<material::Model>,
    pub latest_rate: Option<material_rate::Model>,
}

impl ComponentDetailPage {
    fn body(&self) -> Markup {
        let edit_url = ComponentEditGetRouteTag::new(self.component.id).url();
        let delete_url = ComponentDeleteGetRouteTag::new(self.component.id).url();
        let actions = html! {
            (button_modal_form(ButtonModalForm {
                label: "Edit",
                icon_name: Some("pencil"),
                name: "wo.ComponentEditForm",
                href: &edit_url,
                form_post_url: &edit_url,
                modal_uid: ComponentEditModalKey::ID,
                classes: "btn-outline btn-sm",
                ..Default::default()
            }))
            (button_modal_form(ButtonModalForm {
                label: "Delete",
                icon_name: Some("trash"),
                name: "wo.ComponentDeleteForm",
                href: &delete_url,
                form_post_url: &delete_url,
                modal_uid: ComponentDeleteModalKey::ID,
                classes: "btn-error btn-sm",
                ..Default::default()
            }))
        };

        let shape_name = self.shape.as_ref().map(|s| s.name.clone()).unwrap_or_else(|| "-".into());
        let mat_name = self.material.as_ref().map(|m| m.name.clone()).unwrap_or_else(|| "-".into());
        let density_str = self.material.as_ref().map(|m| format!("{} kg/m³", m.density)).unwrap_or_else(|| "-".into());
        let rate_str = self.latest_rate.as_ref().map(|r| format!("₹ {:.2} / kg", r.rate())).unwrap_or_else(|| "No rate recorded".into());

        let fixed_vars = self.component.fixed_variables_map();
        let fixed_vars_str = if fixed_vars.is_empty() {
            "None".to_string()
        } else {
            fixed_vars.iter().map(|(k, v)| format!("{}: {} mm", k, v)).collect::<Vec<_>>().join(", ")
        };

        let free_vars_str = if let Some(s) = &self.shape {
            let free = self.component.free_variable_names(s);
            if free.is_empty() {
                "None (Fully Specified)".to_string()
            } else {
                format!("{} (in mm)", free.join(", "))
            }
        } else {
            "-".to_string()
        };

        html! {
            (detail(html! {
                (container_column("", html! {
                    (detail_header(DetailHeader {
                        title: &self.component.name,
                        actions,
                    }))
                    (container_row("gap-6", html! {
                        (label("Shape", field_text(FieldText { value: &shape_name, classes: "" })))
                        (label("Material", field_text(FieldText { value: &mat_name, classes: "" })))
                        (label("Material Density", field_text(FieldText { value: &density_str, classes: "" })))
                        (label("Latest Material Rate", field_text(FieldText { value: &rate_str, classes: "" })))
                        (label("Fixed Dimensions (mm)", field_text(FieldText { value: &fixed_vars_str, classes: "" })))
                        (label("Free / Order Variables", field_text(FieldText { value: &free_vars_str, classes: "" })))
                    }))
                    @if let Some(s) = &self.shape {
                        div class="mt-6" {
                            (label("Shape OpenSCAD Template", html! {
                                pre class="p-3 bg-base-200 rounded text-xs font-mono" { (s.openscad_code) }
                            }))
                        }
                    }
                }))
            }))
        }
    }
}

impl RenderAppPane for ComponentDetailPage {
    fn render_pane(&self) -> lariv_rs::components::AppLayoutHtml {
        scaffold_pane(wo_menu("components"), component_crumbs(&self.component.name, self.component.id), self.body())
    }
    fn render_main(&self) -> lariv_rs::components::MainContentHtml {
        scaffold_main(component_crumbs(&self.component.name, self.component.id), self.body())
    }
}

impl RenderTemplate for ComponentDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(&format!("{} — Components", self.component.name), chrome, wo_menu("components"), component_crumbs(&self.component.name, self.component.id), self.body())
    }
}

#[derive(Clone, Generic, Default)]
pub struct ComponentCreateModalPage {
    pub form_name: String,
    pub name: String,
    pub shape_id: Option<i64>,
    pub shape_name: String,
    pub shape_variables: Vec<String>,
    pub all_shapes: Vec<(i64, Vec<String>)>,
    pub material_id: Option<i64>,
    pub material_name: String,
    pub fixed_variables: String,
    pub error: String,
}

impl RenderTemplate for ComponentCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let shape_id_str = self.shape_id.map(|id| id.to_string()).unwrap_or_default();
        let material_id_str = self.material_id.map(|id| id.to_string()).unwrap_or_default();
        let shape_choices: Vec<(String, String)> = self
            .all_shapes
            .iter()
            .map(|(sid, vars)| {
                (
                    sid.to_string(),
                    serde_json::to_string(vars).unwrap_or_else(|_| "[]".into()),
                )
            })
            .collect();
        let ctx = FormCtx::form::<ComponentForm>()
            .value(ComponentFormField::Name, &self.name)
            .value(ComponentFormField::ShapeId, &shape_id_str)
            .display(ComponentFormField::ShapeId, &self.shape_name)
            .value(ComponentFormField::MaterialId, &material_id_str)
            .display(ComponentFormField::MaterialId, &self.material_name)
            .list(ComponentFormField::FixedVariables, &self.shape_variables)
            .choices(ComponentFormField::FixedVariables, &shape_choices)
            .value(ComponentFormField::FixedVariables, &self.fixed_variables);

        modal_keyed::<ComponentCreateModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "New Component" }
                @if !self.error.is_empty() {
                    div class="alert alert-error text-sm mb-4 shadow-sm" {
                        span { (self.error) }
                    }
                }
                (form(FormOpts {
                    attrs: lariv_rs::components::swap::form_hx_post_url::<ComponentCreateModalKey>(
                        &ComponentCreatePostRouteTag.url(),
                    ),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: html! {
                        (ComponentForm::render_inputs(&ctx))
                    },
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Create Component", ..Default::default() }))
                    },
                    ..Default::default()
                }))
            },
        )
    }
}

#[derive(Clone, Generic)]
pub struct ComponentEditModalPage {
    pub id: i64,
    pub form_name: String,
    pub name: String,
    pub shape_id: i64,
    pub shape_name: String,
    pub shape_variables: Vec<String>,
    pub all_shapes: Vec<(i64, Vec<String>)>,
    pub material_id: i64,
    pub material_name: String,
    pub fixed_variables: String,
    pub error: String,
}

impl RenderTemplate for ComponentEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let shape_id_str = self.shape_id.to_string();
        let material_id_str = self.material_id.to_string();
        let shape_choices: Vec<(String, String)> = self
            .all_shapes
            .iter()
            .map(|(sid, vars)| {
                (
                    sid.to_string(),
                    serde_json::to_string(vars).unwrap_or_else(|_| "[]".into()),
                )
            })
            .collect();
        let ctx = FormCtx::form::<ComponentForm>()
            .value(ComponentFormField::Name, &self.name)
            .value(ComponentFormField::ShapeId, &shape_id_str)
            .display(ComponentFormField::ShapeId, &self.shape_name)
            .value(ComponentFormField::MaterialId, &material_id_str)
            .display(ComponentFormField::MaterialId, &self.material_name)
            .list(ComponentFormField::FixedVariables, &self.shape_variables)
            .choices(ComponentFormField::FixedVariables, &shape_choices)
            .value(ComponentFormField::FixedVariables, &self.fixed_variables);

        modal_keyed::<ComponentEditModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit Component" }
                @if !self.error.is_empty() {
                    div class="alert alert-error text-sm mb-4 shadow-sm" {
                        span { (self.error) }
                    }
                }
                (form(FormOpts {
                    attrs: lariv_rs::components::swap::form_hx_post_url::<ComponentEditModalKey>(
                        &ComponentEditPostRouteTag::new(self.id).url(),
                    ),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: html! {
                        (ComponentForm::render_inputs(&ctx))
                    },
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Save Changes", ..Default::default() }))
                    },
                    ..Default::default()
                }))
            },
        )
    }
}

// ==========================================
// 3. SHAPES
// ==========================================

#[derive(Clone, Generic)]
pub struct ShapeListPage {
    pub shapes: Vec<shape::Model>,
    pub path_and_query: String,
}

impl ShapeListPage {
    pub fn render_table(&self) -> Markup {
        let headers = [
            TableColumnHeader { key: "Id", label: "Id", sort_url: None, push_url: false },
            TableColumnHeader { key: "Name", label: "Name", sort_url: None, push_url: false },
            TableColumnHeader { key: "Type", label: "Type", sort_url: None, push_url: false },
            TableColumnHeader { key: "Variables", label: "Variables", sort_url: None, push_url: false },
        ];

        let id_labels: Vec<String> = self.shapes.iter().map(|s| s.id.to_string()).collect();
        let var_labels: Vec<String> = self.shapes.iter().map(|s| s.variable_names_vec().join(", ")).collect();
        let type_cells: Vec<Markup> = self.shapes.iter().map(|s| {
            if s.is_standard() {
                html! { span class="badge badge-primary badge-sm" { "Standard Stock" } }
            } else {
                html! { span class="badge badge-ghost badge-sm" { "Custom CAD" } }
            }
        }).collect();

        let rows: Vec<TableRow> = self.shapes.iter().enumerate().map(|(i, s)| TableRow {
            attrs: row_attr_navigate_route(ShapeDetailRouteTag::new(s.id)),
            cells: vec![
                field_text(FieldText { value: &id_labels[i], classes: "" }),
                field_text(FieldText { value: &s.name, classes: "font-semibold" }),
                type_cells[i].clone(),
                field_text(FieldText { value: &var_labels[i], classes: "font-mono text-xs" }),
            ],
        }).collect();

        let actions = html! {
            (table_create_button::<ShapeTableKey, ShapeCreateModalKey>(
                Some("plus"),
                "btn-square btn-outline btn-sm",
            ))
        };

        data_table_list_refresh::<ShapeTableKey>(
            "Shapes",
            actions,
            &headers,
            &rows,
            html! {},
            &self.path_and_query,
        )
    }
}

impl RenderAppPane for ShapeListPage {
    fn render_pane(&self) -> lariv_rs::components::AppLayoutHtml {
        scaffold_pane(wo_menu("shapes"), shapes_list_crumbs(), self.render_table())
    }
    fn render_main(&self) -> lariv_rs::components::MainContentHtml {
        scaffold_main(shapes_list_crumbs(), self.render_table())
    }
}

impl RenderTemplate for ShapeListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold("Shapes — Work Orders", chrome, wo_menu("shapes"), shapes_list_crumbs(), self.render_table())
    }
}

#[derive(Clone, Generic)]
pub struct ShapeDetailPage {
    pub shape: shape::Model,
}

impl ShapeDetailPage {
    fn body(&self) -> Markup {
        let edit_url = ShapeEditGetRouteTag::new(self.shape.id).url();
        let delete_url = ShapeDeleteGetRouteTag::new(self.shape.id).url();
        let actions = html! {
            (button_modal_form(ButtonModalForm {
                label: "Edit",
                icon_name: Some("pencil"),
                name: "wo.ShapeEditForm",
                href: &edit_url,
                form_post_url: &edit_url,
                modal_uid: ShapeEditModalKey::ID,
                classes: "btn-outline btn-sm",
                ..Default::default()
            }))
            (button_modal_form(ButtonModalForm {
                label: "Delete",
                icon_name: Some("trash"),
                name: "wo.ShapeDeleteForm",
                href: &delete_url,
                form_post_url: &delete_url,
                modal_uid: ShapeDeleteModalKey::ID,
                classes: "btn-error btn-sm",
                ..Default::default()
            }))
        };

        let vars_str = self.shape.variable_names_vec().join(", ");

        html! {
            (detail(html! {
                (container_column("", html! {
                    (detail_header(DetailHeader {
                        title: &self.shape.name,
                        actions,
                    }))
                    (label("Variables (Dimensions in mm)", field_text(FieldText { value: &vars_str, classes: "font-mono font-semibold" })))
                    div class="mt-4" {
                        (label("OpenSCAD Code", html! {
                            pre class="p-3 bg-base-200 rounded text-xs font-mono overflow-x-auto" { (self.shape.openscad_code) }
                        }))
                    }
                }))
            }))
        }
    }
}

impl RenderAppPane for ShapeDetailPage {
    fn render_pane(&self) -> lariv_rs::components::AppLayoutHtml {
        scaffold_pane(wo_menu("shapes"), shape_crumbs(&self.shape.name, self.shape.id), self.body())
    }
    fn render_main(&self) -> lariv_rs::components::MainContentHtml {
        scaffold_main(shape_crumbs(&self.shape.name, self.shape.id), self.body())
    }
}

impl RenderTemplate for ShapeDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(&format!("{} — Shapes", self.shape.name), chrome, wo_menu("shapes"), shape_crumbs(&self.shape.name, self.shape.id), self.body())
    }
}

#[derive(Clone, Generic, Default)]
pub struct ShapeCreateModalPage {
    pub form_name: String,
    pub name: String,
    pub variables: Vec<String>,
    pub openscad_code: String,
    pub error: String,
}

impl RenderTemplate for ShapeCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let ctx = FormCtx::form::<ShapeForm>()
            .value(ShapeFormField::Name, &self.name)
            .list(ShapeFormField::Variables, &self.variables)
            .value(ShapeFormField::OpenscadCode, &self.openscad_code);

        modal_keyed::<ShapeCreateModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-2" { "New Shape" }
                div class="alert alert-info text-xs py-2 px-3 mb-3" {
                    span { "Choose a built-in standard shape preset below, or write custom OpenSCAD code." }
                }
                div class="mb-4" {
                    label class="label pb-1" { span class="label-text font-semibold text-xs uppercase text-base-content/70" { "Standard Presets" } }
                    div class="flex flex-wrap gap-1.5" {
                        button type="button" class="btn btn-outline btn-xs" onclick="setShapePreset('Box / Plate', ['length', 'width', 'thickness'], 'cube([length, width, thickness]);')" { "Box / Plate" }
                        button type="button" class="btn btn-outline btn-xs" onclick="setShapePreset('Cylinder / Round Bar', ['diameter', 'length'], 'cylinder(h=length, r=diameter/2, $fn=64);')" { "Cylinder / Round Bar" }
                        button type="button" class="btn btn-outline btn-xs" onclick="setShapePreset('Hollow Pipe / Tube', ['outer_diameter', 'inner_diameter', 'length'], 'difference() {\\n    cylinder(h=length, r=outer_diameter/2, $fn=64);\\n    cylinder(h=length + 0.001, r=inner_diameter/2, $fn=64);\\n}')" { "Hollow Pipe" }
                        button type="button" class="btn btn-outline btn-xs" onclick="setShapePreset('Hex Bar', ['across_flats', 'length'], 'cylinder(h=length, r=across_flats / sqrt(3), $fn=6);')" { "Hex Bar" }
                        button type="button" class="btn btn-outline btn-xs" onclick="setShapePreset('Flange', ['outer_diameter', 'inner_diameter', 'thickness', 'bolt_diameter', 'num_bolts'], 'difference() {\\n    cylinder(h=thickness, r=outer_diameter/2, $fn=64);\\n    cylinder(h=thickness + 0.001, r=inner_diameter/2, $fn=64);\\n}')" { "Flange" }
                        button type="button" class="btn btn-ghost btn-xs" onclick="setShapePreset('', [], '')" { "Clear (Custom CAD)" }
                    }
                }
                (form(FormOpts {
                    attrs: lariv_rs::components::swap::form_hx_post_url::<ShapeCreateModalKey>(
                        &ShapeCreatePostRouteTag.url(),
                    ),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: html! {
                        (ShapeForm::render_inputs(&ctx))
                        script {
                            (maud::PreEscaped(r#"
function setShapePreset(name, vars, code) {
    const modal = document.getElementById('wo-shape-create-modal');
    const nameEl = modal ? modal.querySelector('input[name=name]') : document.querySelector('input[name=name]');
    if (nameEl) {
        nameEl.value = name;
        nameEl.dispatchEvent(new Event('input', { bubbles: true }));
    }
    const codeEl = modal ? modal.querySelector('textarea[name=openscad_code]') : document.querySelector('textarea[name=openscad_code]');
    if (codeEl) {
        codeEl.value = code;
        codeEl.dispatchEvent(new Event('input', { bubbles: true }));
    }
    const rowInput = modal ? modal.querySelector('[data-list-row-input]') : document.querySelector('[data-list-row-input]');
    const listContainer = rowInput ? rowInput.closest('[x-data]') : null;
    if (listContainer && window.Alpine) {
        const d = Alpine.$data(listContainer);
        if (d) {
            d.items = vars.length > 0
                ? vars.map((v, i) => ({ id: i + 1, value: v }))
                : [{ id: 1, value: '' }];
            d.nextId = vars.length + 1;
        }
    }
}
                            "#))
                        }
                    },
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Create Shape", ..Default::default() }))
                    },
                    ..Default::default()
                }))
            },
        )
    }
}

#[derive(Clone, Generic)]
pub struct ShapeEditModalPage {
    pub id: i64,
    pub form_name: String,
    pub name: String,
    pub openscad_code: String,
    pub variables: Vec<String>,
    pub error: String,
}

impl RenderTemplate for ShapeEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let ctx = FormCtx::form::<ShapeForm>()
            .value(ShapeFormField::Name, &self.name)
            .list(ShapeFormField::Variables, &self.variables)
            .value(ShapeFormField::OpenscadCode, &self.openscad_code);

        modal_keyed::<ShapeEditModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit Shape" }
                (form(FormOpts {
                    attrs: lariv_rs::components::swap::form_hx_post_url::<ShapeEditModalKey>(
                        &ShapeEditPostRouteTag::new(self.id).url(),
                    ),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: ShapeForm::render_inputs(&ctx),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Save Changes", ..Default::default() }))
                    },
                    ..Default::default()
                }))
            },
        )
    }
}

#[derive(Clone, Generic)]
pub struct ShapeSelectPage {
    pub shapes: Vec<shape::Model>,
    pub target_input: String,
    pub path_and_query: String,
}

impl RenderPickerSelect<ShapeSelectTableKey, ShapeSelectModalKey> for ShapeSelectPage {
    fn render_table(&self) -> Markup {
        let target = if self.target_input.is_empty() {
            "shape_id"
        } else {
            self.target_input.as_str()
        };
        let headers = [
            TableColumnHeader { key: "Name", label: "Shape Name", sort_url: None, push_url: false },
            TableColumnHeader { key: "Type", label: "Type", sort_url: None, push_url: false },
            TableColumnHeader { key: "Variables", label: "Dimensions / Variables", sort_url: None, push_url: false },
        ];
        let rows: Vec<TableRow> = self.shapes.iter().map(|s| {
            let type_str = if s.is_standard() { "Standard Stock" } else { "Custom CAD" };
            let vars = s.variable_names_vec();
            let vars_str = vars.join(", ");
            let vars_json = serde_json::to_string(&vars).unwrap_or_else(|_| "[]".into());
            TableRow {
                attrs: row_attr_select_extra(target, &s.id.to_string(), &s.name, &[("variables", &vars_json)]),
                cells: vec![
                    field_text(FieldText { value: &s.name, classes: "font-semibold" }),
                    field_text(FieldText { value: type_str, classes: "text-xs opacity-80" }),
                    field_text(FieldText { value: &vars_str, classes: "font-mono text-xs opacity-70" }),
                ],
            }
        }).collect();

        let actions = html! {
            (picker_create_button::<ShapeCreateModalKey>(
                &self.target_input,
                Some("plus"),
                "btn-square btn-outline btn-sm",
            ))
        };

        data_table_list_refresh::<ShapeSelectTableKey>(
            "Select Shape",
            actions,
            &headers,
            &rows,
            html! {},
            &self.path_and_query,
        )
    }
}

impl RenderTemplate for ShapeSelectPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        self.render_modal().into_inner()
    }
}

// ==========================================
// 4. MATERIALS
// ==========================================

#[derive(Clone, Generic)]
pub struct MaterialListPage {
    pub materials: Vec<(material::Model, Option<material_rate::Model>)>,
    pub path_and_query: String,
}

impl MaterialListPage {
    pub fn render_table(&self) -> Markup {
        let headers = [
            TableColumnHeader { key: "Id", label: "Id", sort_url: None, push_url: false },
            TableColumnHeader { key: "Name", label: "Name", sort_url: None, push_url: false },
            TableColumnHeader { key: "Density", label: "Density (kg/m³)", sort_url: None, push_url: false },
            TableColumnHeader { key: "Rate", label: "Latest Rate", sort_url: None, push_url: false },
        ];

        let id_labels: Vec<String> = self.materials.iter().map(|(m, _)| m.id.to_string()).collect();
        let density_labels: Vec<String> = self.materials.iter().map(|(m, _)| format!("{:.1}", m.density)).collect();
        let rate_labels: Vec<String> = self.materials.iter().map(|(_, r)| r.as_ref().map(|x| format!("₹ {:.2} / kg", x.rate())).unwrap_or_else(|| "-".into())).collect();

        let rows: Vec<TableRow> = self.materials.iter().enumerate().map(|(i, (m, _))| TableRow {
            attrs: row_attr_navigate_route(MaterialDetailRouteTag::new(m.id)),
            cells: vec![
                field_text(FieldText { value: &id_labels[i], classes: "" }),
                field_text(FieldText { value: &m.name, classes: "font-semibold" }),
                field_text(FieldText { value: &density_labels[i], classes: "" }),
                field_text(FieldText { value: &rate_labels[i], classes: "" }),
            ],
        }).collect();

        let actions = html! {
            (table_create_button::<MaterialTableKey, MaterialCreateModalKey>(
                Some("plus"),
                "btn-square btn-outline btn-sm",
            ))
        };

        data_table_list_refresh::<MaterialTableKey>(
            "Materials",
            actions,
            &headers,
            &rows,
            html! {},
            &self.path_and_query,
        )
    }
}

impl RenderAppPane for MaterialListPage {
    fn render_pane(&self) -> lariv_rs::components::AppLayoutHtml {
        scaffold_pane(wo_menu("materials"), materials_list_crumbs(), self.render_table())
    }
    fn render_main(&self) -> lariv_rs::components::MainContentHtml {
        scaffold_main(materials_list_crumbs(), self.render_table())
    }
}

impl RenderTemplate for MaterialListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold("Materials — Work Orders", chrome, wo_menu("materials"), materials_list_crumbs(), self.render_table())
    }
}

#[derive(Clone, Generic)]
pub struct MaterialDetailPage {
    pub material: material::Model,
    pub rates: Vec<material_rate::Model>,
}

impl MaterialDetailPage {
    fn body(&self) -> Markup {
        let edit_url = MaterialEditGetRouteTag::new(self.material.id).url();
        let delete_url = MaterialDeleteGetRouteTag::new(self.material.id).url();
        let actions = html! {
            (button_modal_form(ButtonModalForm {
                label: "Edit",
                icon_name: Some("pencil"),
                name: "wo.MaterialEditForm",
                href: &edit_url,
                form_post_url: &edit_url,
                modal_uid: MaterialEditModalKey::ID,
                classes: "btn-outline btn-sm",
                ..Default::default()
            }))
            (button_modal_form(ButtonModalForm {
                label: "Delete",
                icon_name: Some("trash"),
                name: "wo.MaterialDeleteForm",
                href: &delete_url,
                form_post_url: &delete_url,
                modal_uid: MaterialDeleteModalKey::ID,
                classes: "btn-error btn-sm",
                ..Default::default()
            }))
        };

        let density_str = format!("{} kg/m³", self.material.density);

        html! {
            (detail(html! {
                (container_column("", html! {
                    (detail_header(DetailHeader {
                        title: &self.material.name,
                        actions,
                    }))
                    (label("Density", field_text(FieldText { value: &density_str, classes: "" })))

                    div class="mt-6" {
                        h4 class="font-bold text-md mb-2" { "Rate History" }
                        @if self.rates.is_empty() {
                            p class="text-sm opacity-70" { "No rates recorded yet." }
                        } @else {
                            table class="table table-zebra w-full text-sm" {
                                thead {
                                    tr {
                                        th { "Date / Time (UTC)" }
                                        th { "Rate (INR / kg)" }
                                    }
                                }
                                tbody {
                                    @for r in &self.rates {
                                        tr {
                                            td { (r.datetime.format("%Y-%m-%d %H:%M").to_string()) }
                                            td class="font-semibold" { (format!("₹ {:.2}", r.rate())) }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }))
            }))
        }
    }
}

impl RenderAppPane for MaterialDetailPage {
    fn render_pane(&self) -> lariv_rs::components::AppLayoutHtml {
        scaffold_pane(wo_menu("materials"), material_crumbs(&self.material.name, self.material.id), self.body())
    }
    fn render_main(&self) -> lariv_rs::components::MainContentHtml {
        scaffold_main(material_crumbs(&self.material.name, self.material.id), self.body())
    }
}

impl RenderTemplate for MaterialDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(&format!("{} — Materials", self.material.name), chrome, wo_menu("materials"), material_crumbs(&self.material.name, self.material.id), self.body())
    }
}

#[derive(Clone, Generic)]
pub struct MaterialCreateModalPage {
    pub form_name: String,
    pub error: String,
}

impl RenderTemplate for MaterialCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        modal_keyed::<MaterialCreateModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "New Material" }
                (form(FormOpts {
                    attrs: lariv_rs::components::swap::form_hx_post_url::<MaterialCreateModalKey>(
                        &MaterialCreatePostRouteTag.url(),
                    ),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: html! {
                        div class="form-control mb-3" {
                            label class="label" { span class="label-text" { "Material Name" } }
                            input type="text" name="name" class="input input-bordered w-full" required;
                        }
                        div class="form-control mb-3" {
                            label class="label" { span class="label-text" { "Density (kg/m³, e.g. 7850 for Steel)" } }
                            input type="number" step="0.01" name="density" class="input input-bordered w-full" required;
                        }
                    },
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Create Material", ..Default::default() }))
                    },
                    ..Default::default()
                }))
            },
        )
    }
}

#[derive(Clone, Generic)]
pub struct MaterialEditModalPage {
    pub id: i64,
    pub form_name: String,
    pub name: String,
    pub density: f64,
    pub error: String,
}

impl RenderTemplate for MaterialEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        modal_keyed::<MaterialEditModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit Material" }
                (form(FormOpts {
                    attrs: lariv_rs::components::swap::form_hx_post_url::<MaterialEditModalKey>(
                        &MaterialEditPostRouteTag::new(self.id).url(),
                    ),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: html! {
                        div class="form-control mb-3" {
                            label class="label" { span class="label-text" { "Material Name" } }
                            input type="text" name="name" value=(self.name) class="input input-bordered w-full" required;
                        }
                        div class="form-control mb-3" {
                            label class="label" { span class="label-text" { "Density (kg/m³, e.g. 7850 for Steel)" } }
                            input type="number" step="any" name="density" value=(self.density) class="input input-bordered w-full" required;
                        }
                    },
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Save Changes", ..Default::default() }))
                    },
                    ..Default::default()
                }))
            },
        )
    }
}

#[derive(Clone, Generic)]
pub struct MaterialSelectPage {
    pub materials: Vec<material::Model>,
    pub target_input: String,
    pub path_and_query: String,
}

impl RenderPickerSelect<MaterialSelectTableKey, MaterialSelectModalKey> for MaterialSelectPage {
    fn render_table(&self) -> Markup {
        let target = if self.target_input.is_empty() {
            "material_id"
        } else {
            self.target_input.as_str()
        };
        let headers = [
            TableColumnHeader { key: "Name", label: "Material Name", sort_url: None, push_url: false },
            TableColumnHeader { key: "Density", label: "Density (kg/m³)", sort_url: None, push_url: false },
        ];
        let rows: Vec<TableRow> = self.materials.iter().map(|m| {
            let density_str = format!("{:.1} kg/m³", m.density);
            TableRow {
                attrs: row_attr_select(target, &m.id.to_string(), &m.name),
                cells: vec![
                    field_text(FieldText { value: &m.name, classes: "font-semibold" }),
                    field_text(FieldText { value: &density_str, classes: "font-mono text-xs" }),
                ],
            }
        }).collect();

        let actions = html! {
            (picker_create_button::<MaterialCreateModalKey>(
                &self.target_input,
                Some("plus"),
                "btn-square btn-outline btn-sm",
            ))
        };

        data_table_list_refresh::<MaterialSelectTableKey>(
            "Select Material",
            actions,
            &headers,
            &rows,
            html! {},
            &self.path_and_query,
        )
    }
}

impl RenderTemplate for MaterialSelectPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        self.render_modal().into_inner()
    }
}

// ==========================================
// 5. MATERIAL RATES
// ==========================================

#[derive(Clone, Generic)]
pub struct MaterialRateListPage {
    pub rates: Vec<(material_rate::Model, Option<material::Model>)>,
    pub path_and_query: String,
}

impl MaterialRateListPage {
    pub fn render_table(&self) -> Markup {
        let headers = [
            TableColumnHeader { key: "Id", label: "Id", sort_url: None, push_url: false },
            TableColumnHeader { key: "Material", label: "Material", sort_url: None, push_url: false },
            TableColumnHeader { key: "Rate", label: "Rate (₹ / kg)", sort_url: None, push_url: false },
            TableColumnHeader { key: "Date", label: "Effective Datetime", sort_url: None, push_url: false },
        ];

        let id_labels: Vec<String> = self.rates.iter().map(|(r, _)| r.id.to_string()).collect();
        let mat_labels: Vec<String> = self.rates.iter().map(|(_, m)| m.as_ref().map(|x| x.name.clone()).unwrap_or_else(|| "-".into())).collect();
        let rate_labels: Vec<String> = self.rates.iter().map(|(r, _)| format!("₹ {:.2}", r.rate())).collect();
        let dt_labels: Vec<String> = self.rates.iter().map(|(r, _)| r.datetime.format("%Y-%m-%d %H:%M").to_string()).collect();

        let rows: Vec<TableRow> = self.rates.iter().enumerate().map(|(i, _)| TableRow {
            attrs: lariv_rs::components::HtmlAttrs::new(),
            cells: vec![
                field_text(FieldText { value: &id_labels[i], classes: "" }),
                field_text(FieldText { value: &mat_labels[i], classes: "font-semibold" }),
                field_text(FieldText { value: &rate_labels[i], classes: "" }),
                field_text(FieldText { value: &dt_labels[i], classes: "" }),
            ],
        }).collect();

        let actions = html! {
            (table_create_button::<MaterialRateTableKey, MaterialRateCreateModalKey>(
                Some("plus"),
                "btn-square btn-outline btn-sm",
            ))
        };

        data_table_list_refresh::<MaterialRateTableKey>(
            "Material Rates",
            actions,
            &headers,
            &rows,
            html! {},
            &self.path_and_query,
        )
    }
}

impl RenderAppPane for MaterialRateListPage {
    fn render_pane(&self) -> lariv_rs::components::AppLayoutHtml {
        scaffold_pane(wo_menu("rates"), rates_list_crumbs(), self.render_table())
    }
    fn render_main(&self) -> lariv_rs::components::MainContentHtml {
        scaffold_main(rates_list_crumbs(), self.render_table())
    }
}

impl RenderTemplate for MaterialRateListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold("Material Rates — Work Orders", chrome, wo_menu("rates"), rates_list_crumbs(), self.render_table())
    }
}

#[derive(Clone, Generic)]
pub struct MaterialRateCreateModalPage {
    pub form_name: String,
    pub materials: Vec<material::Model>,
    pub error: String,
}

impl RenderTemplate for MaterialRateCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        modal_keyed::<MaterialRateCreateModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Add Material Rate" }
                (form(FormOpts {
                    attrs: lariv_rs::components::swap::form_hx_post_url::<MaterialRateCreateModalKey>(
                        &MaterialRateCreatePostRouteTag.url(),
                    ),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: html! {
                        div class="form-control mb-3" {
                            label class="label" { span class="label-text" { "Material" } }
                            select name="material_id" class="select select-bordered w-full" required {
                                @for m in &self.materials {
                                    option value=(m.id) { (m.name) }
                                }
                            }
                        }
                        div class="form-control mb-3" {
                            label class="label" { span class="label-text" { "Rate (INR / kg)" } }
                            input type="number" step="0.01" name="rate" class="input input-bordered w-full" required;
                        }
                    },
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Add Rate", ..Default::default() }))
                    },
                    ..Default::default()
                }))
            },
        )
    }
}

// ==========================================
// 6. MACHINES
// ==========================================

#[derive(Clone, Generic)]
pub struct MachineListPage {
    pub machines: Vec<machine::Model>,
    pub path_and_query: String,
}

impl MachineListPage {
    pub fn render_table(&self) -> Markup {
        let headers = [
            TableColumnHeader { key: "Id", label: "Id", sort_url: None, push_url: false },
            TableColumnHeader { key: "Name", label: "Name", sort_url: None, push_url: false },
            TableColumnHeader { key: "Rate", label: "Hourly Rate", sort_url: None, push_url: false },
        ];

        let id_labels: Vec<String> = self.machines.iter().map(|m| m.id.to_string()).collect();
        let rate_labels: Vec<String> = self.machines.iter().map(|m| {
            let (r, p) = m.rate();
            format!("₹ {}.{:02} / hr", r, p)
        }).collect();

        let rows: Vec<TableRow> = self.machines.iter().enumerate().map(|(i, m)| TableRow {
            attrs: row_attr_navigate_route(MachineDetailRouteTag::new(m.id)),
            cells: vec![
                field_text(FieldText { value: &id_labels[i], classes: "" }),
                field_text(FieldText { value: &m.name, classes: "font-semibold" }),
                field_text(FieldText { value: &rate_labels[i], classes: "" }),
            ],
        }).collect();

        let actions = html! {
            (table_create_button::<MachineTableKey, MachineCreateModalKey>(
                Some("plus"),
                "btn-square btn-outline btn-sm",
            ))
        };

        data_table_list_refresh::<MachineTableKey>(
            "Machines",
            actions,
            &headers,
            &rows,
            html! {},
            &self.path_and_query,
        )
    }
}

impl RenderAppPane for MachineListPage {
    fn render_pane(&self) -> lariv_rs::components::AppLayoutHtml {
        scaffold_pane(wo_menu("machines"), machines_list_crumbs(), self.render_table())
    }
    fn render_main(&self) -> lariv_rs::components::MainContentHtml {
        scaffold_main(machines_list_crumbs(), self.render_table())
    }
}

impl RenderTemplate for MachineListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold("Machines — Work Orders", chrome, wo_menu("machines"), machines_list_crumbs(), self.render_table())
    }
}

#[derive(Clone, Generic)]
pub struct MachineDetailPage {
    pub machine: machine::Model,
}

impl MachineDetailPage {
    fn body(&self) -> Markup {
        let edit_url = MachineEditGetRouteTag::new(self.machine.id).url();
        let delete_url = MachineDeleteGetRouteTag::new(self.machine.id).url();
        let actions = html! {
            (button_modal_form(ButtonModalForm {
                label: "Edit",
                icon_name: Some("pencil"),
                name: "wo.MachineEditForm",
                href: &edit_url,
                form_post_url: &edit_url,
                modal_uid: MachineEditModalKey::ID,
                classes: "btn-outline btn-sm",
                ..Default::default()
            }))
            (button_modal_form(ButtonModalForm {
                label: "Delete",
                icon_name: Some("trash"),
                name: "wo.MachineDeleteForm",
                href: &delete_url,
                form_post_url: &delete_url,
                modal_uid: MachineDeleteModalKey::ID,
                classes: "btn-error btn-sm",
                ..Default::default()
            }))
        };

        let (r, p) = self.machine.rate();
        let rate_str = format!("₹ {}.{:02} per hour", r, p);

        html! {
            (detail(html! {
                (container_column("", html! {
                    (detail_header(DetailHeader {
                        title: &self.machine.name,
                        actions,
                    }))
                    (label("Rate (INR / hr)", field_text(FieldText { value: &rate_str, classes: "font-semibold" })))
                }))
            }))
        }
    }
}

impl RenderAppPane for MachineDetailPage {
    fn render_pane(&self) -> lariv_rs::components::AppLayoutHtml {
        scaffold_pane(wo_menu("machines"), machine_crumbs(&self.machine.name, self.machine.id), self.body())
    }
    fn render_main(&self) -> lariv_rs::components::MainContentHtml {
        scaffold_main(machine_crumbs(&self.machine.name, self.machine.id), self.body())
    }
}

impl RenderTemplate for MachineDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(&format!("{} — Machines", self.machine.name), chrome, wo_menu("machines"), machine_crumbs(&self.machine.name, self.machine.id), self.body())
    }
}

#[derive(Clone, Generic)]
pub struct MachineCreateModalPage {
    pub form_name: String,
    pub error: String,
}

impl RenderTemplate for MachineCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        modal_keyed::<MachineCreateModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "New Machine" }
                (form(FormOpts {
                    attrs: lariv_rs::components::swap::form_hx_post_url::<MachineCreateModalKey>(
                        &MachineCreatePostRouteTag.url(),
                    ),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: html! {
                        div class="form-control mb-3" {
                            label class="label" { span class="label-text" { "Machine Name" } }
                            input type="text" name="name" class="input input-bordered w-full" required;
                        }
                        div class="form-control mb-3" {
                            label class="label" { span class="label-text" { "Hourly Rate (INR)" } }
                            input type="number" step="0.01" name="rate" class="input input-bordered w-full" required;
                        }
                    },
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Create Machine", ..Default::default() }))
                    },
                    ..Default::default()
                }))
            },
        )
    }
}

#[derive(Clone, Generic)]
pub struct MachineEditModalPage {
    pub id: i64,
    pub form_name: String,
    pub name: String,
    pub rate: f64,
    pub error: String,
}

impl RenderTemplate for MachineEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        modal_keyed::<MachineEditModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit Machine" }
                (form(FormOpts {
                    attrs: lariv_rs::components::swap::form_hx_post_url::<MachineEditModalKey>(
                        &MachineEditPostRouteTag::new(self.id).url(),
                    ),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: html! {
                        div class="form-control mb-3" {
                            label class="label" { span class="label-text" { "Machine Name" } }
                            input type="text" name="name" value=(self.name) class="input input-bordered w-full" required;
                        }
                        div class="form-control mb-3" {
                            label class="label" { span class="label-text" { "Hourly Rate (INR / hr)" } }
                            input type="number" step="0.01" name="rate" value=(format!("{:.2}", self.rate)) class="input input-bordered w-full" required;
                        }
                    },
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Save Changes", ..Default::default() }))
                    },
                    ..Default::default()
                }))
            },
        )
    }
}

// ==========================================
// 7. PROFORMA INVOICES
// ==========================================

#[derive(Clone, Generic)]
pub struct InvoiceListPage {
    pub invoices: Vec<proforma_invoice::Model>,
    pub customer_names: Vec<String>,
    pub grand_totals: Vec<rust_decimal::Decimal>,
    pub path_and_query: String,
}

impl InvoiceListPage {
    pub fn render_table(&self) -> Markup {
        let headers = [
            TableColumnHeader { key: "Id", label: "Id", sort_url: None, push_url: false },
            TableColumnHeader { key: "InvoiceNumber", label: "Invoice #", sort_url: None, push_url: false },
            TableColumnHeader { key: "Date", label: "Date", sort_url: None, push_url: false },
            TableColumnHeader { key: "Customer", label: "Customer", sort_url: None, push_url: false },
            TableColumnHeader { key: "WorkOrder", label: "Work Order", sort_url: None, push_url: false },
            TableColumnHeader { key: "Total", label: "Total", sort_url: None, push_url: false },
        ];

        let id_labels: Vec<String> = self.invoices.iter().map(|inv| inv.id.to_string()).collect();
        let date_labels: Vec<String> = self.invoices.iter().map(|inv| inv.date.to_string()).collect();
        let wo_labels: Vec<String> = self.invoices.iter().map(|inv| inv.work_order_id.map(|w| format!("#{w}")).unwrap_or_else(|| "-".into())).collect();
        let total_labels: Vec<String> = self.grand_totals.iter().map(|t| format!("₹ {:.2}", t)).collect();

        let rows: Vec<TableRow> = self.invoices.iter().enumerate().map(|(i, inv)| TableRow {
            attrs: row_attr_navigate_route(InvoiceDetailRouteTag::new(inv.id)),
            cells: vec![
                field_text(FieldText { value: &id_labels[i], classes: "" }),
                field_text(FieldText { value: &inv.invoice_number, classes: "font-semibold" }),
                field_text(FieldText { value: &date_labels[i], classes: "" }),
                field_text(FieldText { value: self.customer_names.get(i).map(String::as_str).unwrap_or_default(), classes: "" }),
                field_text(FieldText { value: &wo_labels[i], classes: "" }),
                field_text(FieldText { value: &total_labels[i], classes: "font-semibold" }),
            ],
        }).collect();

        let actions = html! {
            (table_create_button::<InvoiceTableKey, InvoiceCreateModalKey>(
                Some("plus"),
                "btn-square btn-outline btn-sm",
            ))
        };

        data_table_list_refresh::<InvoiceTableKey>(
            "Proforma Invoices",
            actions,
            &headers,
            &rows,
            html! {},
            &self.path_and_query,
        )
    }
}

impl RenderAppPane for InvoiceListPage {
    fn render_pane(&self) -> lariv_rs::components::AppLayoutHtml {
        scaffold_pane(wo_menu("invoices"), invoices_list_crumbs(), self.render_table())
    }
    fn render_main(&self) -> lariv_rs::components::MainContentHtml {
        scaffold_main(invoices_list_crumbs(), self.render_table())
    }
}

impl RenderTemplate for InvoiceListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold("Proforma Invoices — Work Orders", chrome, wo_menu("invoices"), invoices_list_crumbs(), self.render_table())
    }
}

#[derive(Clone, Generic)]
pub struct InvoiceDetailPage {
    pub invoice: proforma_invoice::Model,
    pub machine_lines: Vec<proforma_invoice_machine_line::Model>,
    pub material_lines: Vec<proforma_invoice_material_line::Model>,
    pub grand_total: rust_decimal::Decimal,
    pub customer_name: String,
    pub work_order_number: String,
}

impl InvoiceDetailPage {
    fn body(&self) -> Markup {
        let edit_url = InvoiceEditGetRouteTag::new(self.invoice.id).url();
        let delete_url = InvoiceDeleteGetRouteTag::new(self.invoice.id).url();
        let pdf_url = InvoicePdfRouteTag::new(self.invoice.id).url();
        let actions = html! {
            (button_modal_form(ButtonModalForm {
                label: "Edit",
                icon_name: Some("pencil"),
                name: "wo.InvoiceEditForm",
                href: &edit_url,
                form_post_url: &edit_url,
                modal_uid: InvoiceEditModalKey::ID,
                classes: "btn-outline btn-sm",
                ..Default::default()
            }))
            (button_modal_form(ButtonModalForm {
                label: "Delete",
                icon_name: Some("trash"),
                name: "wo.InvoiceDeleteForm",
                href: &delete_url,
                form_post_url: &delete_url,
                modal_uid: InvoiceDeleteModalKey::ID,
                classes: "btn-error btn-sm",
                ..Default::default()
            }))
            a class="btn btn-outline btn-sm" href=(pdf_url) target="_blank" rel="noopener" {
                "Download PDF"
            }
        };

        let date_str = self.invoice.date.to_string();
        let cust_str = self.customer_name.clone();
        let wo_str = if self.work_order_number.is_empty() {
            self.invoice.work_order_id.map(|w| format!("#{w}")).unwrap_or_else(|| "None".into())
        } else {
            self.work_order_number.clone()
        };

        html! {
            (detail(html! {
                (container_column("", html! {
                    (detail_header(DetailHeader {
                        title: &format!("Proforma Invoice {}", self.invoice.invoice_number),
                        actions,
                    }))
                    (container_row("gap-6", html! {
                        (label("Invoice Date", field_text(FieldText { value: &date_str, classes: "" })))
                        (label("Customer", field_text(FieldText { value: &cust_str, classes: "" })))
                        (label("Work Order", field_text(FieldText { value: &wo_str, classes: "" })))
                    }))

                    // Machine Lines Table
                    div class="mt-6" {
                        h4 class="font-bold text-md mb-2" { "Machine Operations" }
                        @if self.machine_lines.is_empty() {
                            p class="text-sm opacity-70" { "No machine lines on this invoice." }
                        } @else {
                            table class="table table-zebra w-full text-sm" {
                                thead {
                                    tr {
                                        th { "Operation / Machine" }
                                        th { "Time Used" }
                                        th { "Rate / hr" }
                                        th { "Amount" }
                                    }
                                }
                                tbody {
                                    @for l in &self.machine_lines {
                                        @let (r, p) = l.rate();
                                        tr {
                                            td { (l.name) }
                                            td { (format!("{:.2} hrs", l.time_used.as_hours_f64())) }
                                            td { (format!("₹ {}.{:02}", r, p)) }
                                            td class="font-semibold" { (format!("₹ {:.2}", l.line_total())) }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Material Lines Table
                    div class="mt-6" {
                        h4 class="font-bold text-md mb-2" { "Material Lines" }
                        @if self.material_lines.is_empty() {
                            p class="text-sm opacity-70" { "No material lines on this invoice." }
                        } @else {
                            table class="table table-zebra w-full text-sm" {
                                thead {
                                    tr {
                                        th { "Material" }
                                        th { "Qty (kg)" }
                                        th { "Rate / kg" }
                                        th { "Amount" }
                                    }
                                }
                                tbody {
                                    @for l in &self.material_lines {
                                        @let (r, p) = l.rate();
                                        tr {
                                            td { (l.name) }
                                            td { (format!("{:.3} kg", l.qty_decimal)) }
                                            td { (format!("₹ {}.{:02}", r, p)) }
                                            td class="font-semibold" { (format!("₹ {:.2}", l.line_total())) }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Total
                    div class="mt-6 p-4 bg-base-200 rounded flex justify-between items-center" {
                        span class="text-lg font-bold" { "Grand Total" }
                        span class="text-xl font-extrabold text-primary" { (format!("₹ {:.2}", self.grand_total)) }
                    }
                }))
            }))
        }
    }
}

impl RenderAppPane for InvoiceDetailPage {
    fn render_pane(&self) -> lariv_rs::components::AppLayoutHtml {
        scaffold_pane(wo_menu("invoices"), invoice_crumbs(&self.invoice.invoice_number, self.invoice.id), self.body())
    }
    fn render_main(&self) -> lariv_rs::components::MainContentHtml {
        scaffold_main(invoice_crumbs(&self.invoice.invoice_number, self.invoice.id), self.body())
    }
}

impl RenderTemplate for InvoiceDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(&format!("{} — Invoices", self.invoice.invoice_number), chrome, wo_menu("invoices"), invoice_crumbs(&self.invoice.invoice_number, self.invoice.id), self.body())
    }
}

#[derive(Clone, Generic)]
pub struct InvoiceCreateModalPage {
    pub form_name: String,
    pub date: String,
    pub customer_name: String,
    pub work_order_name: String,
    pub material_lines_json: String,
    pub machine_lines_json: String,
    pub materials_json: String,
    pub machines_json: String,
    pub error: String,
}

impl RenderTemplate for InvoiceCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let material_lines_val = if self.material_lines_json.is_empty() { "[]" } else { &self.material_lines_json };
        let machine_lines_val = if self.machine_lines_json.is_empty() { "[]" } else { &self.machine_lines_json };
        let ctx = FormCtx::form::<InvoiceForm>()
            .value(InvoiceFormField::Date, &self.date)
            .value(InvoiceFormField::CustomerId, "")
            .display(InvoiceFormField::CustomerId, &self.customer_name)
            .value(InvoiceFormField::WorkOrderId, "")
            .display(InvoiceFormField::WorkOrderId, &self.work_order_name)
            .value(InvoiceFormField::MaterialLines, material_lines_val)
            .display(InvoiceFormField::MaterialLines, &self.materials_json)
            .value(InvoiceFormField::MachineLines, machine_lines_val)
            .display(InvoiceFormField::MachineLines, &self.machines_json);

        let modal_classes = format!("!max-w-6xl !w-11/12 {}", self.form_name);
        modal_keyed::<InvoiceCreateModalKey>(
            &modal_classes,
            html! {
                h3 class="font-bold text-lg mb-4" { "New Proforma Invoice" }
                @if !self.error.is_empty() {
                    div class="alert alert-error text-sm mb-4 shadow-sm" {
                        span { (self.error) }
                    }
                }
                (form(FormOpts {
                    attrs: lariv_rs::components::swap::form_hx_post_url::<InvoiceCreateModalKey>(
                        &InvoiceCreatePostRouteTag.url(),
                    ),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: html! {
                        (InvoiceForm::render_inputs(&ctx))
                    },
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Create Invoice", ..Default::default() }))
                    },
                    ..Default::default()
                }))
            },
        )
    }
}

#[derive(Clone, Generic)]
pub struct InvoiceEditModalPage {
    pub id: i64,
    pub form_name: String,
    pub invoice_number: String,
    pub date: String,
    pub customer_id: i64,
    pub customer_name: String,
    pub work_order_id: Option<i64>,
    pub work_order_name: String,
    pub material_lines_json: String,
    pub machine_lines_json: String,
    pub materials_json: String,
    pub machines_json: String,
    pub error: String,
}

impl RenderTemplate for InvoiceEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let cust_id_str = self.customer_id.to_string();
        let wo_id_str = self.work_order_id.map(|w| w.to_string()).unwrap_or_default();
        let material_lines_val = if self.material_lines_json.is_empty() { "[]" } else { &self.material_lines_json };
        let machine_lines_val = if self.machine_lines_json.is_empty() { "[]" } else { &self.machine_lines_json };
        let ctx = FormCtx::form::<InvoiceForm>()
            .value(InvoiceFormField::InvoiceNumber, &self.invoice_number)
            .value(InvoiceFormField::Date, &self.date)
            .value(InvoiceFormField::CustomerId, &cust_id_str)
            .display(InvoiceFormField::CustomerId, &self.customer_name)
            .value(InvoiceFormField::WorkOrderId, &wo_id_str)
            .display(InvoiceFormField::WorkOrderId, &self.work_order_name)
            .value(InvoiceFormField::MaterialLines, material_lines_val)
            .display(InvoiceFormField::MaterialLines, &self.materials_json)
            .value(InvoiceFormField::MachineLines, machine_lines_val)
            .display(InvoiceFormField::MachineLines, &self.machines_json);
        let delete_url = InvoiceDeleteGetRouteTag::new(self.id).url();

        let modal_classes = format!("!max-w-6xl !w-11/12 {}", self.form_name);
        modal_keyed::<InvoiceEditModalKey>(
            &modal_classes,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit Proforma Invoice" }
                @if !self.error.is_empty() {
                    div class="alert alert-error text-sm mb-4 shadow-sm" {
                        span { (self.error) }
                    }
                }
                (form(FormOpts {
                    attrs: lariv_rs::components::swap::form_hx_post_url::<InvoiceEditModalKey>(
                        &InvoiceEditPostRouteTag::new(self.id).url(),
                    ),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: html! {
                        (InvoiceForm::render_inputs(&ctx))
                    },
                    actions: html! {
                        (button_modal_form(ButtonModalForm {
                            label: "Delete",
                            icon_name: Some("trash"),
                            name: "wo.InvoiceDeleteForm",
                            href: &delete_url,
                            form_post_url: &delete_url,
                            modal_uid: InvoiceDeleteModalKey::ID,
                            classes: "btn-error btn-sm",
                            ..Default::default()
                        }))
                        (button_submit(ButtonSubmit { label: "Save Changes", ..Default::default() }))
                    },
                    ..Default::default()
                }))
            },
        )
    }
}

// ==========================================
// 8. DELETE CONFIRMATION
// ==========================================

#[derive(Clone, Generic)]
pub struct ConfirmDeleteModalPage {
    pub modal_uid: String,
    pub title: String,
    pub message: String,
    pub post_url: String,
    pub error: String,
}

impl RenderTemplate for ConfirmDeleteModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let target = format!("#{}", self.modal_uid);
        modal(lariv_rs::components::Modal {
            uid: &self.modal_uid,
            children: delete_confirmation(DeleteConfirmation {
                title: &self.title,
                message: &self.message,
                attrs: lariv_rs::components::swap::form_hx_post_selector(&self.post_url, &target),
                form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                ..Default::default()
            }),
            ..Default::default()
        })
    }
}

// ==========================================
// 9. PDF PREFERENCES
// ==========================================

#[derive(Clone, Generic)]
pub struct WorkOrdersPreferencesPage {
    pub draft_work_order_pdf_template: String,
    pub proforma_invoice_pdf_template: String,
    pub error: String,
}

fn pdf_template_editor(
    label: &str,
    field_id: &str,
    value: &str,
    preview_post_url: &str,
    default_hint: &str,
    rows: u32,
) -> Markup {
    html! {
        div class="form-control mb-8" {
            label class="label" {
                span class="label-text font-bold text-base" { (label) }
            }
            (code_editor_input(CodeEditorInput {
                label: "",
                name: field_id,
                value,
                id: field_id,
                language: "typst",
                rows,
                max_height: "26rem",
                required: false,
                classes: "",
                attrs: Default::default(),
                hint: None,
            }))
            textarea id=(format!("{field_id}-default")) hidden readonly { (default_hint) }
            div class="flex justify-end gap-2 mt-2" {
                button type="button" class="btn btn-ghost btn-sm"
                    onclick=(format!(
                        "if (confirm('This will overwrite the template with the default example template. Continue?')) {{ const ta = document.getElementById('{}'); const def = document.getElementById('{}-default'); if (!ta || !def) return; ta.value = def.value; const root = ta.closest('[data-code-editor-root]'); if (root) {{ root.dispatchEvent(new CustomEvent('code-editor:set', {{ detail: {{ value: def.value }} }})); }} else {{ ta.dispatchEvent(new Event('change', {{ bubbles: true }})); }} }}",
                        field_id, field_id
                    )) {
                    "Use default template"
                }
                (PreEscaped(format!(
                    r#"<button type="button" class="btn btn-outline btn-sm" hx-post="{url}" hx-target="{target}" hx-swap="{swap}" hx-include="closest form" hx-push-url="false">Preview sample PDF</button>"#,
                    url = escape_attr(preview_post_url),
                    target = escape_attr(HTMX_TARGET_BODY_MODAL),
                    swap = escape_attr(HTMX_SWAP_BODY_MODAL),
                )))
            }
        }
    }
}

impl WorkOrdersPreferencesPage {
    fn body(&self) -> Markup {
        form(lariv_rs::components::FormOpts {
                attrs: lariv_rs::components::form_hx_post_main_url(&WorkOrdersPrefsPostRouteTag.url()),
                title: "Work Orders PDF Preferences",
                subtitle: "Configure the PDF templates used for draft work orders and proforma invoices. Templates are Jinja2 (Minijinja) that render Typst source; the result is compiled to PDF.",
                form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                inputs: html! {
                    (pdf_template_editor(
                        "Draft Work Order PDF Template",
                        "draft_work_order_pdf_template",
                        &self.draft_work_order_pdf_template,
                        &WorkOrderPdfPreviewPostRouteTag.url(),
                        crate::work_orders::pdf_templates::DEFAULT_DRAFT_WORK_ORDER_PDF_TEMPLATE,
                        18,
                    ))
                    (pdf_template_editor(
                        "Proforma Invoice PDF Template",
                        "proforma_invoice_pdf_template",
                        &self.proforma_invoice_pdf_template,
                        &InvoicePdfPreviewPostRouteTag.url(),
                        crate::work_orders::pdf_templates::DEFAULT_PROFORMA_INVOICE_PDF_TEMPLATE,
                        18,
                    ))
                },
                actions: html! {
                    (button_submit(ButtonSubmit {
                        label: "Save Preferences",
                        ..Default::default()
                    }))
                },
                ..Default::default()
            },
        )
    }
}

impl RenderAppPane for WorkOrdersPreferencesPage {
    fn render_pane(&self) -> lariv_rs::components::AppLayoutHtml {
        scaffold_pane(wo_menu("preferences"), work_orders_prefs_crumbs(), self.body())
    }
    fn render_main(&self) -> lariv_rs::components::MainContentHtml {
        scaffold_main(work_orders_prefs_crumbs(), self.body())
    }
}

impl RenderTemplate for WorkOrdersPreferencesPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Work Orders PDF Preferences",
            chrome,
            wo_menu("preferences"),
            work_orders_prefs_crumbs(),
            self.body(),
        )
    }
}
