use frunk::Generic;
use lariv_rs::{
    components::{
        ButtonModalForm, ButtonPost, ButtonSubmit, CodeEditorInput, DeleteConfirmation,
        DetailHeader, FieldLink, FieldText, FormOpts, HTMX_SWAP_BODY_MODAL, HTMX_TARGET_BODY_MODAL,
        LayoutMain, LayoutSidebar, ManyToManyItem, ShellChrome, ShellScaffold, SlotCapability,
        SlotRegistrar, SwapKey, TableColumnHeader, TableRow, attrs::escape_attr, button_modal_form,
        button_modal_route, button_post, button_submit, code_editor_input, container_column,
        container_row, data_table_list_refresh, delete_confirmation, detail, detail_header,
        field_link, field_text, form, form_hx_post_route, label, label_hint, layout_main,
        layout_sidebar, modal, modal_keyed, row_attr_navigate_route, row_attr_select,
        row_attr_select_extra, shell_scaffold, table_create_button,
    },
    http::ProvideRequestCaps,
    picker::RenderPickerSelect,
    template::{RenderAppPane, RenderTemplate, TemplateCapability, TemplateOf, TemplateRegistrar},
    web::modal_create_post_query,
};
use maud::PreEscaped;
use maud::{Markup, html};

use crate::machinery_schedule::logic::format_job_duration;

use lariv_rs::html_form::{CsrfToken, FieldRender, FormCtx, FormFieldKey, HtmlForm};

use rust_decimal::Decimal;

use super::crumbs::*;
use super::entities::{
    component, draft_work_order, draft_work_order_machine_line, draft_work_order_material_line,
    quotation, quotation_machine_line, quotation_material_line, work_order, work_order_line,
    work_order_machine_line,
};
use super::forms::{
    ComponentForm, ComponentFormField, DraftWorkOrderForm, DraftWorkOrderFormField,
    DraftWorkOrderLineForm, DraftWorkOrderLineFormField, DraftWorkOrderMachineLineForm,
    DraftWorkOrderMachineLineFormField, InvoiceForm, InvoiceFormField, LineEditorDisplayKey,
    WorkOrdersPreferencesForm, WorkOrdersPreferencesFormField, draft_work_order_form_hx_post,
    lines_form_hx_post, schema_entries_from_json,
};
use super::keys::*;
use super::routes::*;
use crate::machinery_schedule::routes::JobDetailRouteTag;

fn formula_preview(s: &str) -> String {
    let t = s.trim();
    if t.is_empty() {
        "—".into()
    } else if t.len() > 40 {
        format!("{}…", &t[..40])
    } else {
        t.to_string()
    }
}

fn schema_label(variables: &serde_json::Value) -> String {
    let entries = schema_entries_from_json(variables);
    if entries.is_empty() {
        "—".into()
    } else {
        entries.join(", ")
    }
}

fn order_number_display(s: &str) -> &str {
    if s.trim().is_empty() { "—" } else { s }
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

        InvoiceListPageIdx: InvoiceListPageTag => InvoiceListPage,
        InvoiceDetailPageIdx: InvoiceDetailPageTag => InvoiceDetailPage,
        InvoiceCreateModalPageIdx: InvoiceCreateModalPageTag => InvoiceCreateModalPage,
        InvoiceEditModalPageIdx: InvoiceEditModalPageTag => InvoiceEditModalPage,

        IssuedWorkOrderListPageIdx: IssuedWorkOrderListPageTag => IssuedWorkOrderListPage,
        IssuedWorkOrderDetailPageIdx: IssuedWorkOrderDetailPageTag => IssuedWorkOrderDetailPage,
        IssuedWorkOrderDeleteModalPageIdx: IssuedWorkOrderDeleteModalPageTag => IssuedWorkOrderDeleteModalPage,

        ConfirmDeleteModalPageIdx: ConfirmDeleteModalPageTag => ConfirmDeleteModalPage,
        WorkOrderLineEditModalPageIdx: WorkOrderLineEditModalPageTag => WorkOrderLineEditModalPage,
        WorkOrderMachineLineEditModalPageIdx: WorkOrderMachineLineEditModalPageTag => WorkOrderMachineLineEditModalPage,
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

fn tab_nav_link(href: &str, active: bool, label: &str) -> Markup {
    let cls = if active { "tab tab-active" } else { "tab" };
    let nav = lariv_rs::components::nav_content_attrs(href);
    html! {
        (PreEscaped(format!(
            r#"<a class="{cls}" href="{href}"{attrs}>"#,
            cls = escape_attr(cls),
            href = escape_attr(href),
            attrs = nav.as_string(),
        )))
        (label)
        (PreEscaped("</a>"))
    }
}

fn work_order_hub_tabs(active: &str) -> Markup {
    html! {
        div class="tabs tabs-boxed mb-4" {
            (tab_nav_link(&work_orders_tab_url("drafts"), active == "drafts", "Drafts"))
            (tab_nav_link(&work_orders_tab_url("issued"), active == "issued", "Work Orders"))
        }
    }
}

fn work_order_hub_body(active: &str, table: Markup) -> Markup {
    html! {
        (work_order_hub_tabs(active))
        (table)
    }
}

// ==========================================
// 1. WORK ORDERS
// ==========================================

#[derive(Clone, Generic)]
pub struct WorkOrderListPage {
    pub orders: Vec<(draft_work_order::Model, usize, Decimal)>,
    pub path_and_query: String,
}

impl WorkOrderListPage {
    pub fn render_table(&self) -> Markup {
        let headers = [
            TableColumnHeader {
                key: "Id",
                label: "Id",
                sort_url: None,
                push_url: false,
            },
            TableColumnHeader {
                key: "OrderNumber",
                label: "Order #",
                sort_url: None,
                push_url: false,
            },
            TableColumnHeader {
                key: "CustomerId",
                label: "Customer ID",
                sort_url: None,
                push_url: false,
            },
            TableColumnHeader {
                key: "Lines",
                label: "Lines",
                sort_url: None,
                push_url: false,
            },
            TableColumnHeader {
                key: "TotalAmount",
                label: "Total Cost",
                sort_url: None,
                push_url: false,
            },
        ];

        let id_labels: Vec<String> = self
            .orders
            .iter()
            .map(|(o, _, _)| o.id.to_string())
            .collect();
        let cust_labels: Vec<String> = self
            .orders
            .iter()
            .map(|(o, _, _)| o.customer_id.to_string())
            .collect();
        let lines_labels: Vec<String> = self
            .orders
            .iter()
            .map(|(_, count, _)| count.to_string())
            .collect();
        let total_labels: Vec<String> = self
            .orders
            .iter()
            .map(|(_, _, total)| format!("₹ {:.2}", total))
            .collect();

        let rows: Vec<TableRow> = self
            .orders
            .iter()
            .enumerate()
            .map(|(i, (o, _, _))| TableRow {
                attrs: row_attr_navigate_route(WorkOrderDetailRouteTag::new(o.id)),
                cells: vec![
                    field_text(FieldText {
                        value: &id_labels[i],
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: order_number_display(&o.order_number),
                        classes: "font-semibold",
                    }),
                    field_text(FieldText {
                        value: &cust_labels[i],
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &lines_labels[i],
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &total_labels[i],
                        classes: "font-mono font-semibold",
                    }),
                ],
            })
            .collect();

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

    fn body(&self) -> Markup {
        work_order_hub_body("drafts", self.render_table())
    }
}

impl RenderAppPane for WorkOrderListPage {
    fn render_pane(&self) -> lariv_rs::components::AppLayoutHtml {
        scaffold_pane(wo_menu("orders"), work_orders_list_crumbs(), self.body())
    }
    fn render_main(&self) -> lariv_rs::components::MainContentHtml {
        scaffold_main(work_orders_list_crumbs(), self.body())
    }
}

impl RenderTemplate for WorkOrderListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Work Orders",
            chrome,
            wo_menu("orders"),
            work_orders_list_crumbs(),
            self.body(),
        )
    }
}

#[derive(Clone, Generic)]
pub struct WorkOrderDetailPage {
    pub order: draft_work_order::Model,
    pub lines: Vec<(
        draft_work_order_material_line::Model,
        String,
        String,
        Decimal,
    )>,
    pub machine_lines: Vec<(
        draft_work_order_machine_line::Model,
        String,
        String,
        Decimal,
    )>,
    pub customer_name: Option<String>,
    pub quotation_number: Option<String>,
    pub total_amount: Decimal,
}

impl WorkOrderDetailPage {
    fn body(&self) -> Markup {
        let edit_url = WorkOrderEditGetRouteTag::new(self.order.id).url();
        let actions = html! {
            (button_post(ButtonPost {
                label: "Convert to Work Order",
                action: &WorkOrderConvertPostRouteTag::new(self.order.id).path(),
                icon_name: Some("check"),
                classes: "btn-primary btn-sm",
                ..Default::default()
            }))
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
            (button_modal_route(
                WorkOrderPdfModalRouteTag::new(self.order.id),
                "PDF",
                "btn-outline btn-sm",
            ))
        };

        let cust_label = match &self.customer_name {
            Some(name) => format!("{} (#{})", name, self.order.customer_id),
            None => format!("#{}", self.order.customer_id),
        };
        let quotation_href = self
            .order
            .quotation_id
            .map(|id| InvoiceDetailRouteTag::new(id).url());
        let quotation_label = self.quotation_number.clone().unwrap_or_else(|| {
            self.order
                .quotation_id
                .map(|id| format!("#{id}"))
                .unwrap_or_default()
        });
        let lines_count_str = self.lines.len().to_string();
        let machine_lines_count_str = self.machine_lines.len().to_string();
        let total_str = format!("₹ {:.2}", self.total_amount);
        let duration_str = format_job_duration(self.order.duration);
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
                        @if let Some(href) = &quotation_href {
                            (label("Quotation", field_link(FieldLink { href, label: &quotation_label, classes: "" })))
                        }
                        (label("Duration", field_text(FieldText { value: &duration_str, classes: "" })))
                        (label("Material Lines", field_text(FieldText { value: &lines_count_str, classes: "" })))
                        (label("Machine Lines", field_text(FieldText { value: &machine_lines_count_str, classes: "" })))
                        (label("Total Cost", field_text(FieldText { value: &total_str, classes: "font-mono font-bold text-primary" })))
                    }))

                    div class="mt-8" {
                        div class="mb-3" {
                            h4 class="font-bold text-lg" { "Material Lines" }
                        }

                        @if self.lines.is_empty() {
                            div class="p-8 text-center text-sm text-base-content/60 bg-base-200/50 rounded-lg border border-base-200" {
                                "No line items added yet."
                            }
                        } @else {
                            div class="overflow-x-auto min-w-0 w-full border border-base-200 rounded-lg" {
                                table class="table table-zebra w-full min-w-max text-sm" {
                                    thead {
                                        tr {
                                            th class="w-12" { "#" }
                                            th { "Component" }
                                            th { "Variables" }
                                            th { "Taxes" }
                                            th class="text-right" { "Pre-tax (₹)" }
                                            th class="text-right" { "Total (₹)" }
                                        }
                                    }
                                    tbody {
                                        @for (idx, (l, comp_name, tax_labels, taxed_total)) in self.lines.iter().enumerate() {
                                            tr {
                                                td class="opacity-60" { (idx + 1) }
                                                td class="font-semibold" { (comp_name) }
                                                td class="font-mono text-xs" { (l.format_variables_display()) }
                                                td class="text-sm" { (tax_labels) }
                                                td class="text-right font-mono" { (format!("₹ {:.2}", l.final_cost)) }
                                                td class="text-right font-mono font-bold text-primary" { (format!("₹ {:.2}", taxed_total)) }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    div class="mt-8" {
                        div class="mb-3" {
                            h4 class="font-bold text-lg" { "Machine Lines" }
                        }

                        @if self.machine_lines.is_empty() {
                            div class="p-8 text-center text-sm text-base-content/60 bg-base-200/50 rounded-lg border border-base-200" {
                                "No machine lines added yet."
                            }
                        } @else {
                            div class="overflow-x-auto min-w-0 w-full border border-base-200 rounded-lg" {
                                table class="table table-zebra w-full min-w-max text-sm" {
                                    thead {
                                        tr {
                                            th class="w-12" { "#" }
                                            th { "Machine" }
                                            th { "Variables" }
                                            th { "Taxes" }
                                            th class="text-right" { "Pre-tax (₹)" }
                                            th class="text-right" { "Total (₹)" }
                                        }
                                    }
                                    tbody {
                                        @for (idx, (l, machine_name, tax_labels, taxed_total)) in self.machine_lines.iter().enumerate() {
                                            tr {
                                                td class="opacity-60" { (idx + 1) }
                                                td class="font-semibold" { (machine_name) }
                                                td class="font-mono text-xs" { (l.format_variables_display()) }
                                                td class="text-sm" { (tax_labels) }
                                                td class="text-right font-mono" { (format!("₹ {:.2}", l.line_total())) }
                                                td class="text-right font-mono font-bold text-primary" { (format!("₹ {:.2}", taxed_total)) }
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
        scaffold_pane(
            wo_menu("orders"),
            work_order_crumbs(
                order_number_display(&self.order.order_number),
                self.order.id,
            ),
            self.body(),
        )
    }
    fn render_main(&self) -> lariv_rs::components::MainContentHtml {
        scaffold_main(
            work_order_crumbs(
                order_number_display(&self.order.order_number),
                self.order.id,
            ),
            self.body(),
        )
    }
}

impl RenderTemplate for WorkOrderDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        let page_title = if self.order.order_number.trim().is_empty() {
            "Draft Work Order — Work Orders".to_string()
        } else {
            format!("Order {} — Work Orders", self.order.order_number)
        };
        app_scaffold(
            &page_title,
            chrome,
            wo_menu("orders"),
            work_order_crumbs(
                order_number_display(&self.order.order_number),
                self.order.id,
            ),
            self.body(),
        )
    }
}

#[derive(Clone, Generic, Default)]
pub struct WorkOrderCreateModalPage {
    pub form_name: String,
    pub order_number: String,
    pub customer_id: Option<i64>,
    pub customer_name: String,
    pub duration: String,
    pub items_json: String,
    pub components_json: String,
    pub machine_lines_json: String,
    pub machines_json: String,
    pub taxes_json: String,
    pub error: String,
}

impl RenderTemplate for WorkOrderCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let cust_id_str = self
            .customer_id
            .map(|id| id.to_string())
            .unwrap_or_default();
        let items_val = if self.items_json.is_empty() {
            "[]"
        } else {
            &self.items_json
        };
        let machine_lines_val = if self.machine_lines_json.is_empty() {
            "[]"
        } else {
            &self.machine_lines_json
        };
        let ctx = FormCtx::form::<DraftWorkOrderForm>(CsrfToken::current())
            .value(DraftWorkOrderFormField::OrderNumber, &self.order_number)
            .value(DraftWorkOrderFormField::CustomerId, &cust_id_str)
            .display(DraftWorkOrderFormField::CustomerId, &self.customer_name)
            .value(DraftWorkOrderFormField::Duration, &self.duration)
            .value(DraftWorkOrderFormField::Items, items_val)
            .display(DraftWorkOrderFormField::Items, &self.components_json)
            .value(DraftWorkOrderFormField::MachineLines, machine_lines_val)
            .display(DraftWorkOrderFormField::MachineLines, &self.machines_json)
            .display(LineEditorDisplayKey::TaxesData, &self.taxes_json);

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
                (form(&CsrfToken::current(), FormOpts {
                    attrs: draft_work_order_form_hx_post::<DraftWorkOrderCreateModalKey>(
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
    pub duration: String,
    pub items_json: String,
    pub components_json: String,
    pub machine_lines_json: String,
    pub machines_json: String,
    pub taxes_json: String,
    pub error: String,
}

impl RenderTemplate for WorkOrderEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let cust_id_str = self.customer_id.to_string();
        let items_val = if self.items_json.is_empty() {
            "[]"
        } else {
            &self.items_json
        };
        let machine_lines_val = if self.machine_lines_json.is_empty() {
            "[]"
        } else {
            &self.machine_lines_json
        };
        let ctx = FormCtx::form::<DraftWorkOrderForm>(CsrfToken::current())
            .value(DraftWorkOrderFormField::OrderNumber, &self.order_number)
            .value(DraftWorkOrderFormField::CustomerId, &cust_id_str)
            .display(DraftWorkOrderFormField::CustomerId, &self.customer_name)
            .value(DraftWorkOrderFormField::Duration, &self.duration)
            .value(DraftWorkOrderFormField::Items, items_val)
            .display(DraftWorkOrderFormField::Items, &self.components_json)
            .value(DraftWorkOrderFormField::MachineLines, machine_lines_val)
            .display(DraftWorkOrderFormField::MachineLines, &self.machines_json)
            .display(LineEditorDisplayKey::TaxesData, &self.taxes_json);
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
                (form(&CsrfToken::current(), FormOpts {
                    attrs: draft_work_order_form_hx_post::<DraftWorkOrderEditModalKey>(
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
    pub extra_data: String,
    pub tax_items: Vec<ManyToManyItem>,
    pub error: String,
}

impl RenderTemplate for WorkOrderLineEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let wo_id_str = self.draft_work_order_id.to_string();
        let comp_id_str = self.component_id.to_string();
        let vars_str = if self.variables.is_empty() {
            "{}"
        } else {
            &self.variables
        };
        let ctx = FormCtx::form::<DraftWorkOrderLineForm>(CsrfToken::current())
            .value(DraftWorkOrderLineFormField::DraftWorkOrderId, &wo_id_str)
            .display(
                DraftWorkOrderLineFormField::DraftWorkOrderId,
                &self.draft_work_order_label,
            )
            .value(DraftWorkOrderLineFormField::ComponentId, &comp_id_str)
            .display(
                DraftWorkOrderLineFormField::ComponentId,
                &self.component_label,
            )
            .value(DraftWorkOrderLineFormField::Variables, vars_str)
            .value(DraftWorkOrderLineFormField::ExtraData, &self.extra_data)
            .m2m(DraftWorkOrderLineFormField::Taxes, &self.tax_items);

        modal_keyed::<DraftWorkOrderLineEditModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit Material Line" }
                @if !self.error.is_empty() {
                    div class="alert alert-error text-sm mb-4 shadow-sm" {
                        span { (self.error) }
                    }
                }
                (form(&CsrfToken::current(), FormOpts {
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
    pub variables: String,
    pub tax_items: Vec<ManyToManyItem>,
    pub error: String,
}

impl RenderTemplate for WorkOrderMachineLineEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let wo_id_str = self.draft_work_order_id.to_string();
        let machine_id_str = self.machine_id.to_string();
        let vars_str = if self.variables.is_empty() {
            "{}"
        } else {
            &self.variables
        };
        let ctx = FormCtx::form::<DraftWorkOrderMachineLineForm>(CsrfToken::current())
            .value(
                DraftWorkOrderMachineLineFormField::DraftWorkOrderId,
                &wo_id_str,
            )
            .display(
                DraftWorkOrderMachineLineFormField::DraftWorkOrderId,
                &self.draft_work_order_label,
            )
            .value(
                DraftWorkOrderMachineLineFormField::MachineId,
                &machine_id_str,
            )
            .display(
                DraftWorkOrderMachineLineFormField::MachineId,
                &self.machine_label,
            )
            .value(DraftWorkOrderMachineLineFormField::Variables, vars_str)
            .m2m(DraftWorkOrderMachineLineFormField::Taxes, &self.tax_items);

        modal_keyed::<DraftWorkOrderMachineLineEditModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit Machine Line" }
                @if !self.error.is_empty() {
                    div class="alert alert-error text-sm mb-4 shadow-sm" {
                        span { (self.error) }
                    }
                }
                (form(&CsrfToken::current(), FormOpts {
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
            },
        )
    }
}

#[derive(Clone, Generic)]
pub struct WorkOrderSelectPage {
    pub orders: Vec<draft_work_order::Model>,
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
            TableColumnHeader {
                key: "OrderNumber",
                label: "Order #",
                sort_url: None,
                push_url: false,
            },
            TableColumnHeader {
                key: "CustomerId",
                label: "Customer ID",
                sort_url: None,
                push_url: false,
            },
        ];
        let rows: Vec<TableRow> = self
            .orders
            .iter()
            .map(|o| TableRow {
                attrs: row_attr_select(
                    target,
                    &o.id.to_string(),
                    order_number_display(&o.order_number),
                ),
                cells: vec![
                    field_text(FieldText {
                        value: order_number_display(&o.order_number),
                        classes: "font-semibold",
                    }),
                    field_text(FieldText {
                        value: &o.customer_id.to_string(),
                        classes: "font-mono",
                    }),
                ],
            })
            .collect();

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
    pub components: Vec<component::Model>,
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
            TableColumnHeader {
                key: "Name",
                label: "Name",
                sort_url: None,
                push_url: false,
            },
            TableColumnHeader {
                key: "Variables",
                label: "Variables",
                sort_url: None,
                push_url: false,
            },
            TableColumnHeader {
                key: "CostFormula",
                label: "Cost Formula",
                sort_url: None,
                push_url: false,
            },
        ];
        let extras: Vec<(String, String, String, String)> = self
            .components
            .iter()
            .map(|c| {
                (
                    serde_json::to_string(&c.variables).unwrap_or_else(|_| "{}".into()),
                    c.cost_formula.clone(),
                    c.weight_formula.clone(),
                    schema_label(&c.variables),
                )
            })
            .collect();
        let formula_labels: Vec<String> = self
            .components
            .iter()
            .map(|c| formula_preview(&c.cost_formula))
            .collect();
        let rows: Vec<TableRow> = self
            .components
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let (vars_s, cost_s, weight_s, schema_s) = &extras[i];
                TableRow {
                    attrs: row_attr_select_extra(
                        target,
                        &c.id.to_string(),
                        &c.name,
                        &[
                            ("variables", vars_s.as_str()),
                            ("cost_formula", cost_s.as_str()),
                            ("weight_formula", weight_s.as_str()),
                        ],
                    ),
                    cells: vec![
                        field_text(FieldText {
                            value: &c.name,
                            classes: "font-semibold",
                        }),
                        field_text(FieldText {
                            value: schema_s,
                            classes: "font-mono text-xs",
                        }),
                        field_text(FieldText {
                            value: &formula_labels[i],
                            classes: "font-mono text-xs",
                        }),
                    ],
                }
            })
            .collect();

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
    pub components: Vec<component::Model>,
    pub path_and_query: String,
}

impl ComponentListPage {
    pub fn render_table(&self) -> Markup {
        let headers = [
            TableColumnHeader {
                key: "Id",
                label: "Id",
                sort_url: None,
                push_url: false,
            },
            TableColumnHeader {
                key: "Name",
                label: "Name",
                sort_url: None,
                push_url: false,
            },
            TableColumnHeader {
                key: "Variables",
                label: "Variables",
                sort_url: None,
                push_url: false,
            },
            TableColumnHeader {
                key: "CostFormula",
                label: "Cost Formula",
                sort_url: None,
                push_url: false,
            },
        ];

        let id_labels: Vec<String> = self.components.iter().map(|c| c.id.to_string()).collect();
        let schema_labels: Vec<String> = self
            .components
            .iter()
            .map(|c| schema_label(&c.variables))
            .collect();
        let formula_labels: Vec<String> = self
            .components
            .iter()
            .map(|c| formula_preview(&c.cost_formula))
            .collect();

        let rows: Vec<TableRow> = self
            .components
            .iter()
            .enumerate()
            .map(|(i, c)| TableRow {
                attrs: row_attr_navigate_route(ComponentDetailRouteTag::new(c.id)),
                cells: vec![
                    field_text(FieldText {
                        value: &id_labels[i],
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &c.name,
                        classes: "font-semibold",
                    }),
                    field_text(FieldText {
                        value: &schema_labels[i],
                        classes: "font-mono text-xs",
                    }),
                    field_text(FieldText {
                        value: &formula_labels[i],
                        classes: "font-mono text-xs",
                    }),
                ],
            })
            .collect();

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
        scaffold_pane(
            wo_menu("components"),
            components_list_crumbs(),
            self.render_table(),
        )
    }
    fn render_main(&self) -> lariv_rs::components::MainContentHtml {
        scaffold_main(components_list_crumbs(), self.render_table())
    }
}

impl RenderTemplate for ComponentListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Components — KDS Quotations",
            chrome,
            wo_menu("components"),
            components_list_crumbs(),
            self.render_table(),
        )
    }
}

#[derive(Clone, Generic)]
pub struct ComponentDetailPage {
    pub component: component::Model,
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

        let schema_str = schema_label(&self.component.variables);
        let cost_str = if self.component.cost_formula.trim().is_empty() {
            "—".to_string()
        } else {
            self.component.cost_formula.clone()
        };
        let weight_str = if self.component.weight_formula.trim().is_empty() {
            "—".to_string()
        } else {
            self.component.weight_formula.clone()
        };

        html! {
            (detail(html! {
                (container_column("", html! {
                    (detail_header(DetailHeader {
                        title: &self.component.name,
                        actions,
                    }))
                    (container_row("gap-6", html! {
                        (label("Variables", field_text(FieldText { value: &schema_str, classes: "font-mono text-sm" })))
                    }))
                    div class="mt-6" {
                        (label("Cost Formula", html! {
                            pre class="p-3 bg-base-200 rounded text-xs font-mono whitespace-pre-wrap" { (cost_str) }
                        }))
                    }
                    div class="mt-4" {
                        (label("Weight Formula", html! {
                            pre class="p-3 bg-base-200 rounded text-xs font-mono whitespace-pre-wrap" { (weight_str) }
                        }))
                    }
                }))
            }))
        }
    }
}

impl RenderAppPane for ComponentDetailPage {
    fn render_pane(&self) -> lariv_rs::components::AppLayoutHtml {
        scaffold_pane(
            wo_menu("components"),
            component_crumbs(&self.component.name, self.component.id),
            self.body(),
        )
    }
    fn render_main(&self) -> lariv_rs::components::MainContentHtml {
        scaffold_main(
            component_crumbs(&self.component.name, self.component.id),
            self.body(),
        )
    }
}

impl RenderTemplate for ComponentDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            &format!("{} — Components", self.component.name),
            chrome,
            wo_menu("components"),
            component_crumbs(&self.component.name, self.component.id),
            self.body(),
        )
    }
}

#[derive(Clone, Generic, Default)]
pub struct ComponentCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub target_input: String,
    pub name: String,
    pub variables: Vec<String>,
    pub cost_formula: String,
    pub weight_formula: String,
    pub error: String,
}

impl RenderTemplate for ComponentCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let ctx = FormCtx::form::<ComponentForm>(CsrfToken::current())
            .value(ComponentFormField::Name, &self.name)
            .list(ComponentFormField::Variables, &self.variables)
            .value(ComponentFormField::CostFormula, &self.cost_formula)
            .value(ComponentFormField::WeightFormula, &self.weight_formula);

        modal_keyed::<ComponentCreateModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "New Component" }
                @if !self.error.is_empty() {
                    div class="alert alert-error text-sm mb-4 shadow-sm" {
                        span { (self.error) }
                    }
                }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: lariv_rs::components::swap::form_hx_post_url::<ComponentCreateModalKey>(
                        &modal_create_post_query(
                            ComponentCreatePostRouteTag,
                            &self.form_name,
                            &self.refresh_table,
                            &self.target_input,
                        ),
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
    pub variables: Vec<String>,
    pub cost_formula: String,
    pub weight_formula: String,
    pub error: String,
}

impl RenderTemplate for ComponentEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let ctx = FormCtx::form::<ComponentForm>(CsrfToken::current())
            .value(ComponentFormField::Name, &self.name)
            .list(ComponentFormField::Variables, &self.variables)
            .value(ComponentFormField::CostFormula, &self.cost_formula)
            .value(ComponentFormField::WeightFormula, &self.weight_formula);

        modal_keyed::<ComponentEditModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit Component" }
                @if !self.error.is_empty() {
                    div class="alert alert-error text-sm mb-4 shadow-sm" {
                        span { (self.error) }
                    }
                }
                (form(&CsrfToken::current(), FormOpts {
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

#[derive(Clone, Generic)]
pub struct InvoiceListPage {
    pub invoices: Vec<quotation::Model>,
    pub customer_names: Vec<String>,
    pub grand_totals: Vec<rust_decimal::Decimal>,
    pub path_and_query: String,
}

impl InvoiceListPage {
    pub fn render_table(&self) -> Markup {
        let headers = [
            TableColumnHeader {
                key: "Id",
                label: "Id",
                sort_url: None,
                push_url: false,
            },
            TableColumnHeader {
                key: "InvoiceNumber",
                label: "Quotation #",
                sort_url: None,
                push_url: false,
            },
            TableColumnHeader {
                key: "Date",
                label: "Date",
                sort_url: None,
                push_url: false,
            },
            TableColumnHeader {
                key: "Customer",
                label: "Customer",
                sort_url: None,
                push_url: false,
            },
            TableColumnHeader {
                key: "Total",
                label: "Total",
                sort_url: None,
                push_url: false,
            },
        ];

        let id_labels: Vec<String> = self.invoices.iter().map(|inv| inv.id.to_string()).collect();
        let date_labels: Vec<String> = self
            .invoices
            .iter()
            .map(|inv| inv.date.to_string())
            .collect();
        let total_labels: Vec<String> = self
            .grand_totals
            .iter()
            .map(|t| format!("₹ {:.2}", t))
            .collect();

        let rows: Vec<TableRow> = self
            .invoices
            .iter()
            .enumerate()
            .map(|(i, inv)| TableRow {
                attrs: row_attr_navigate_route(InvoiceDetailRouteTag::new(inv.id)),
                cells: vec![
                    field_text(FieldText {
                        value: &id_labels[i],
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &inv.invoice_number,
                        classes: "font-semibold",
                    }),
                    field_text(FieldText {
                        value: &date_labels[i],
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: self
                            .customer_names
                            .get(i)
                            .map(String::as_str)
                            .unwrap_or_default(),
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &total_labels[i],
                        classes: "font-semibold",
                    }),
                ],
            })
            .collect();

        let actions = html! {
            (table_create_button::<InvoiceTableKey, InvoiceCreateModalKey>(
                Some("plus"),
                "btn-square btn-outline btn-sm",
            ))
        };

        data_table_list_refresh::<InvoiceTableKey>(
            "Quotations",
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
        scaffold_pane(
            wo_menu("invoices"),
            invoices_list_crumbs(),
            self.render_table(),
        )
    }
    fn render_main(&self) -> lariv_rs::components::MainContentHtml {
        scaffold_main(invoices_list_crumbs(), self.render_table())
    }
}

impl RenderTemplate for InvoiceListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Quotations — KDS Quotations",
            chrome,
            wo_menu("invoices"),
            invoices_list_crumbs(),
            self.render_table(),
        )
    }
}

#[derive(Clone, Generic)]
pub struct InvoiceDetailPage {
    pub invoice: quotation::Model,
    pub machine_lines: Vec<(
        quotation_machine_line::Model,
        String,
        String,
        rust_decimal::Decimal,
    )>,
    pub material_lines: Vec<(
        quotation_material_line::Model,
        String,
        String,
        rust_decimal::Decimal,
    )>,
    pub grand_total: rust_decimal::Decimal,
    pub customer_name: String,
}

impl InvoiceDetailPage {
    fn body(&self) -> Markup {
        let edit_url = InvoiceEditGetRouteTag::new(self.invoice.id).url();
        let create_wo_path = InvoiceCreateWorkOrderPostRouteTag::new(self.invoice.id).path();
        let actions = html! {
            (button_post(ButtonPost {
                label: "Create Work Order",
                action: &create_wo_path,
                icon_name: Some("plus"),
                classes: "btn-primary btn-sm",
                ..Default::default()
            }))
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
            (button_modal_route(
                InvoicePdfModalRouteTag::new(self.invoice.id),
                "PDF",
                "btn-outline btn-sm",
            ))
        };

        let date_str = self.invoice.date.to_string();
        let cust_str = self.customer_name.clone();
        let duration_str = format_job_duration(self.invoice.duration);

        html! {
            (detail(html! {
                (container_column("", html! {
                    (detail_header(DetailHeader {
                        title: &format!("Quotation {}", self.invoice.invoice_number),
                        actions,
                    }))
                    (container_row("gap-6", html! {
                        (label("Quotation Date", field_text(FieldText { value: &date_str, classes: "" })))
                        (label("Customer", field_text(FieldText { value: &cust_str, classes: "" })))
                        (label("Duration", field_text(FieldText { value: &duration_str, classes: "" })))
                    }))

                    // Machine Lines Table
                    div class="mt-6 min-w-0 w-full" {
                        h4 class="font-bold text-md mb-2" { "Machine Operations" }
                        @if self.machine_lines.is_empty() {
                            p class="text-sm opacity-70" { "No machine lines on this quotation." }
                        } @else {
                            div class="overflow-x-auto min-w-0 w-full" {
                            table class="table table-zebra w-full min-w-max text-sm" {
                                thead {
                                    tr {
                                        th { "Machine" }
                                        th { "Variables" }
                                        th { "Taxes" }
                                        th { "Pre-tax" }
                                        th { "Amount" }
                                    }
                                }
                                tbody {
                                    @for (l, machine_name, tax_labels, taxed_total) in &self.machine_lines {
                                        tr {
                                            td { (machine_name) }
                                            td class="font-mono text-xs" { (l.format_variables_display()) }
                                            td { (tax_labels) }
                                            td class="font-semibold" { (format!("₹ {:.2}", l.line_total())) }
                                            td class="font-semibold" { (format!("₹ {:.2}", taxed_total)) }
                                        }
                                    }
                                }
                            }
                            }
                        }
                    }

                    // Material Lines Table
                    div class="mt-6 min-w-0 w-full" {
                        h4 class="font-bold text-md mb-2" { "Material Lines" }
                        @if self.material_lines.is_empty() {
                            p class="text-sm opacity-70" { "No material lines on this quotation." }
                        } @else {
                            div class="overflow-x-auto min-w-0 w-full" {
                            table class="table table-zebra w-full min-w-max text-sm" {
                                thead {
                                    tr {
                                        th { "Component" }
                                        th { "Variables" }
                                        th { "Taxes" }
                                        th { "Pre-tax" }
                                        th { "Amount" }
                                    }
                                }
                                tbody {
                                    @for (l, comp_name, tax_labels, taxed_total) in &self.material_lines {
                                        tr {
                                            td { (comp_name) }
                                            td class="font-mono text-xs" { (l.format_variables_display()) }
                                            td { (tax_labels) }
                                            td class="font-semibold" { (format!("₹ {:.2}", l.final_cost)) }
                                            td class="font-semibold" { (format!("₹ {:.2}", taxed_total)) }
                                        }
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
        scaffold_pane(
            wo_menu("invoices"),
            invoice_crumbs(&self.invoice.invoice_number, self.invoice.id),
            self.body(),
        )
    }
    fn render_main(&self) -> lariv_rs::components::MainContentHtml {
        scaffold_main(
            invoice_crumbs(&self.invoice.invoice_number, self.invoice.id),
            self.body(),
        )
    }
}

impl RenderTemplate for InvoiceDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            &format!("{} — Quotations", self.invoice.invoice_number),
            chrome,
            wo_menu("invoices"),
            invoice_crumbs(&self.invoice.invoice_number, self.invoice.id),
            self.body(),
        )
    }
}

#[derive(Clone, Generic)]
pub struct IssuedWorkOrderListPage {
    pub orders: Vec<work_order::Model>,
    pub customer_names: Vec<String>,
    pub job_labels: Vec<String>,
    pub line_counts: Vec<usize>,
    pub totals: Vec<Decimal>,
    pub path_and_query: String,
}

impl IssuedWorkOrderListPage {
    pub fn render_table(&self) -> Markup {
        let headers = [
            TableColumnHeader {
                key: "Id",
                label: "Id",
                sort_url: None,
                push_url: false,
            },
            TableColumnHeader {
                key: "OrderNumber",
                label: "Order #",
                sort_url: None,
                push_url: false,
            },
            TableColumnHeader {
                key: "Customer",
                label: "Customer",
                sort_url: None,
                push_url: false,
            },
            TableColumnHeader {
                key: "Job",
                label: "Job",
                sort_url: None,
                push_url: false,
            },
            TableColumnHeader {
                key: "Lines",
                label: "Lines",
                sort_url: None,
                push_url: false,
            },
            TableColumnHeader {
                key: "Total",
                label: "Total Cost",
                sort_url: None,
                push_url: false,
            },
        ];

        let id_labels: Vec<String> = self.orders.iter().map(|o| o.id.to_string()).collect();
        let line_labels: Vec<String> = self.line_counts.iter().map(|n| n.to_string()).collect();
        let total_labels: Vec<String> = self.totals.iter().map(|t| format!("₹ {:.2}", t)).collect();

        let rows: Vec<TableRow> = self
            .orders
            .iter()
            .enumerate()
            .map(|(i, o)| TableRow {
                attrs: row_attr_navigate_route(IssuedWorkOrderDetailRouteTag::new(o.id)),
                cells: vec![
                    field_text(FieldText {
                        value: &id_labels[i],
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: order_number_display(&o.order_number),
                        classes: "font-semibold",
                    }),
                    field_text(FieldText {
                        value: self
                            .customer_names
                            .get(i)
                            .map(String::as_str)
                            .unwrap_or_default(),
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: self.job_labels.get(i).map(String::as_str).unwrap_or("—"),
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &line_labels[i],
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &total_labels[i],
                        classes: "font-semibold",
                    }),
                ],
            })
            .collect();

        data_table_list_refresh::<IssuedWorkOrderTableKey>(
            "Work Orders",
            html! {},
            &headers,
            &rows,
            html! {},
            &self.path_and_query,
        )
    }

    fn body(&self) -> Markup {
        work_order_hub_body("issued", self.render_table())
    }
}

impl RenderAppPane for IssuedWorkOrderListPage {
    fn render_pane(&self) -> lariv_rs::components::AppLayoutHtml {
        scaffold_pane(
            wo_menu("issued"),
            issued_work_orders_list_crumbs(),
            self.body(),
        )
    }
    fn render_main(&self) -> lariv_rs::components::MainContentHtml {
        scaffold_main(issued_work_orders_list_crumbs(), self.body())
    }
}

impl RenderTemplate for IssuedWorkOrderListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Work Orders — KDS Quotations",
            chrome,
            wo_menu("issued"),
            issued_work_orders_list_crumbs(),
            self.body(),
        )
    }
}

#[derive(Clone, Generic)]
pub struct IssuedWorkOrderDetailPage {
    pub order: work_order::Model,
    pub lines: Vec<(work_order_line::Model, String, String, Decimal)>,
    pub machine_lines: Vec<(work_order_machine_line::Model, String, String, Decimal)>,
    pub customer_name: Option<String>,
    pub quotation_number: Option<String>,
    pub job_name: Option<String>,
    pub total_amount: Decimal,
}

impl IssuedWorkOrderDetailPage {
    fn body(&self) -> Markup {
        let delete_url = IssuedWorkOrderDeleteGetRouteTag::new(self.order.id).url();
        let new_draft_path = IssuedWorkOrderNewDraftPostRouteTag::new(self.order.id).path();
        let actions = html! {
            (button_post(ButtonPost {
                label: "New Draft Work Order",
                action: &new_draft_path,
                icon_name: Some("plus"),
                classes: "btn-primary btn-sm",
                ..Default::default()
            }))
            (button_modal_form(ButtonModalForm {
                label: "Delete",
                icon_name: Some("trash"),
                name: "wo.IssuedWorkOrderDeleteForm",
                href: &delete_url,
                form_post_url: &delete_url,
                modal_uid: IssuedWorkOrderDeleteModalKey::ID,
                classes: "btn-error btn-outline btn-sm",
                ..Default::default()
            }))
        };

        let cust_label = match &self.customer_name {
            Some(name) => format!("{} (#{})", name, self.order.customer_id),
            None => format!("#{}", self.order.customer_id),
        };
        let quotation_href = self
            .order
            .quotation_id
            .map(|id| InvoiceDetailRouteTag::new(id).url());
        let quotation_label = self.quotation_number.clone().unwrap_or_else(|| {
            self.order
                .quotation_id
                .map(|id| format!("#{id}"))
                .unwrap_or_default()
        });
        let job_href = self.order.job_id.map(|id| JobDetailRouteTag::new(id).url());
        let job_label = self.job_name.clone().unwrap_or_else(|| {
            self.order
                .job_id
                .map(|id| format!("#{id}"))
                .unwrap_or_default()
        });
        let lines_count_str = self.lines.len().to_string();
        let machine_lines_count_str = self.machine_lines.len().to_string();
        let total_str = format!("₹ {:.2}", self.total_amount);
        let duration_str = format_job_duration(self.order.duration);
        let title_str = if self.order.order_number.trim().is_empty() {
            "Work Order".to_string()
        } else {
            format!("Work Order {}", self.order.order_number)
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
                        @if let Some(href) = &quotation_href {
                            (label("Quotation", field_link(FieldLink { href, label: &quotation_label, classes: "" })))
                        }
                        @if let Some(href) = &job_href {
                            (label("Job", field_link(FieldLink { href, label: &job_label, classes: "" })))
                        }
                        (label("Duration", field_text(FieldText { value: &duration_str, classes: "" })))
                        (label("Material Lines", field_text(FieldText { value: &lines_count_str, classes: "" })))
                        (label("Machine Lines", field_text(FieldText { value: &machine_lines_count_str, classes: "" })))
                        (label("Total Cost", field_text(FieldText { value: &total_str, classes: "font-mono font-bold text-primary" })))
                    }))

                    div class="mt-8" {
                        div class="mb-3" {
                            h4 class="font-bold text-lg" { "Material Lines" }
                        }
                        @if self.lines.is_empty() {
                            div class="p-8 text-center text-sm text-base-content/60 bg-base-200/50 rounded-lg border border-base-200" {
                                "No line items."
                            }
                        } @else {
                            div class="overflow-x-auto min-w-0 w-full border border-base-200 rounded-lg" {
                                table class="table table-zebra w-full min-w-max text-sm" {
                                    thead {
                                        tr {
                                            th class="w-12" { "#" }
                                            th { "Component" }
                                            th { "Variables" }
                                            th { "Taxes" }
                                            th class="text-right" { "Pre-tax (₹)" }
                                            th class="text-right" { "Total (₹)" }
                                        }
                                    }
                                    tbody {
                                        @for (idx, (l, comp_name, tax_labels, taxed_total)) in self.lines.iter().enumerate() {
                                            tr {
                                                td class="opacity-60" { (idx + 1) }
                                                td class="font-semibold" { (comp_name) }
                                                td class="font-mono text-xs" { (l.format_variables_display()) }
                                                td class="text-sm" { (tax_labels) }
                                                td class="text-right font-mono" { (format!("₹ {:.2}", l.final_cost)) }
                                                td class="text-right font-mono font-bold text-primary" { (format!("₹ {:.2}", taxed_total)) }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    div class="mt-8" {
                        div class="mb-3" {
                            h4 class="font-bold text-lg" { "Machine Lines" }
                        }
                        @if self.machine_lines.is_empty() {
                            div class="p-8 text-center text-sm text-base-content/60 bg-base-200/50 rounded-lg border border-base-200" {
                                "No machine lines."
                            }
                        } @else {
                            div class="overflow-x-auto min-w-0 w-full border border-base-200 rounded-lg" {
                                table class="table table-zebra w-full min-w-max text-sm" {
                                    thead {
                                        tr {
                                            th class="w-12" { "#" }
                                            th { "Machine" }
                                            th { "Variables" }
                                            th { "Taxes" }
                                            th class="text-right" { "Pre-tax (₹)" }
                                            th class="text-right" { "Total (₹)" }
                                        }
                                    }
                                    tbody {
                                        @for (idx, (l, machine_name, tax_labels, taxed_total)) in self.machine_lines.iter().enumerate() {
                                            tr {
                                                td class="opacity-60" { (idx + 1) }
                                                td class="font-semibold" { (machine_name) }
                                                td class="font-mono text-xs" { (l.format_variables_display()) }
                                                td class="text-sm" { (tax_labels) }
                                                td class="text-right font-mono" { (format!("₹ {:.2}", l.line_total())) }
                                                td class="text-right font-mono font-bold text-primary" { (format!("₹ {:.2}", taxed_total)) }
                                            }
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

impl RenderAppPane for IssuedWorkOrderDetailPage {
    fn render_pane(&self) -> lariv_rs::components::AppLayoutHtml {
        scaffold_pane(
            wo_menu("issued"),
            issued_work_order_crumbs(
                order_number_display(&self.order.order_number),
                self.order.id,
            ),
            self.body(),
        )
    }
    fn render_main(&self) -> lariv_rs::components::MainContentHtml {
        scaffold_main(
            issued_work_order_crumbs(
                order_number_display(&self.order.order_number),
                self.order.id,
            ),
            self.body(),
        )
    }
}

impl RenderTemplate for IssuedWorkOrderDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        let page_title = if self.order.order_number.trim().is_empty() {
            "Work Order — Work Orders".to_string()
        } else {
            format!("{} — Work Orders", self.order.order_number)
        };
        app_scaffold(
            &page_title,
            chrome,
            wo_menu("issued"),
            issued_work_order_crumbs(
                order_number_display(&self.order.order_number),
                self.order.id,
            ),
            self.body(),
        )
    }
}

#[derive(Clone, Generic)]
pub struct InvoiceCreateModalPage {
    pub form_name: String,
    pub invoice_number: String,
    pub date: String,
    pub customer_id: Option<i64>,
    pub customer_name: String,
    pub duration: String,
    pub material_lines_json: String,
    pub machine_lines_json: String,
    pub components_json: String,
    pub machines_json: String,
    pub taxes_json: String,
    pub error: String,
}

impl RenderTemplate for InvoiceCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let cust_id_str = self
            .customer_id
            .map(|id| id.to_string())
            .unwrap_or_default();
        let material_lines_val = if self.material_lines_json.is_empty() {
            "[]"
        } else {
            &self.material_lines_json
        };
        let machine_lines_val = if self.machine_lines_json.is_empty() {
            "[]"
        } else {
            &self.machine_lines_json
        };
        let ctx = FormCtx::form::<InvoiceForm>(CsrfToken::current())
            .value(InvoiceFormField::InvoiceNumber, &self.invoice_number)
            .value(InvoiceFormField::Date, &self.date)
            .value(InvoiceFormField::CustomerId, &cust_id_str)
            .display(InvoiceFormField::CustomerId, &self.customer_name)
            .value(InvoiceFormField::Duration, &self.duration)
            .value(InvoiceFormField::MaterialLines, material_lines_val)
            .display(InvoiceFormField::MaterialLines, &self.components_json)
            .value(InvoiceFormField::MachineLines, machine_lines_val)
            .display(InvoiceFormField::MachineLines, &self.machines_json)
            .display(LineEditorDisplayKey::TaxesData, &self.taxes_json);

        let modal_classes = format!("!max-w-6xl !w-11/12 {}", self.form_name);
        modal_keyed::<InvoiceCreateModalKey>(
            &modal_classes,
            html! {
                h3 class="font-bold text-lg mb-4" { "New Quotation" }
                @if !self.error.is_empty() {
                    div class="alert alert-error text-sm mb-4 shadow-sm" {
                        span { (self.error) }
                    }
                }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: lines_form_hx_post::<InvoiceCreateModalKey>(
                        &InvoiceCreatePostRouteTag.url(),
                    ),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: html! {
                        (InvoiceForm::render_inputs(&ctx))
                    },
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Create Quotation", ..Default::default() }))
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
    pub duration: String,
    pub material_lines_json: String,
    pub machine_lines_json: String,
    pub components_json: String,
    pub machines_json: String,
    pub taxes_json: String,
    pub error: String,
}

impl RenderTemplate for InvoiceEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let cust_id_str = self.customer_id.to_string();
        let material_lines_val = if self.material_lines_json.is_empty() {
            "[]"
        } else {
            &self.material_lines_json
        };
        let machine_lines_val = if self.machine_lines_json.is_empty() {
            "[]"
        } else {
            &self.machine_lines_json
        };
        let ctx = FormCtx::form::<InvoiceForm>(CsrfToken::current())
            .value(InvoiceFormField::InvoiceNumber, &self.invoice_number)
            .value(InvoiceFormField::Date, &self.date)
            .value(InvoiceFormField::CustomerId, &cust_id_str)
            .display(InvoiceFormField::CustomerId, &self.customer_name)
            .value(InvoiceFormField::Duration, &self.duration)
            .value(InvoiceFormField::MaterialLines, material_lines_val)
            .display(InvoiceFormField::MaterialLines, &self.components_json)
            .value(InvoiceFormField::MachineLines, machine_lines_val)
            .display(InvoiceFormField::MachineLines, &self.machines_json)
            .display(LineEditorDisplayKey::TaxesData, &self.taxes_json);
        let delete_url = InvoiceDeleteGetRouteTag::new(self.id).url();

        let modal_classes = format!("!max-w-6xl !w-11/12 {}", self.form_name);
        modal_keyed::<InvoiceEditModalKey>(
            &modal_classes,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit Quotation" }
                @if !self.error.is_empty() {
                    div class="alert alert-error text-sm mb-4 shadow-sm" {
                        span { (self.error) }
                    }
                }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: lines_form_hx_post::<InvoiceEditModalKey>(
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

fn related_doc_cell(name: &str, url: &str) -> Markup {
    if url.is_empty() {
        field_text(FieldText {
            value: name,
            classes: "",
        })
    } else {
        field_link(FieldLink {
            href: url,
            label: name,
            classes: "",
        })
    }
}

#[derive(Clone)]
pub struct IssuedWorkOrderDeleteItem {
    pub kind: String,
    pub label: String,
    pub url: String,
}

#[derive(Generic)]
pub struct IssuedWorkOrderDeleteModalPage {
    pub id: i64,
    pub items: Vec<IssuedWorkOrderDeleteItem>,
    pub can_delete: bool,
    pub error: Option<String>,
}

impl RenderTemplate for IssuedWorkOrderDeleteModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let body = if !self.can_delete {
            let err = self.error.as_deref().unwrap_or("Delete is unavailable.");
            html! {
                h3 class="font-bold text-lg" { "Delete unavailable" }
                div class="alert alert-error my-2 text-sm" { (err) }
            }
        } else {
            html! {
                h2 class="text-xl font-bold text-error" { "Delete work order" }
                p class="my-2" { "The following documents will be permanently deleted:" }
                ul class="list-disc ml-6 my-3 space-y-1" {
                    @for item in &self.items {
                        li {
                            span class="font-semibold" { (item.kind) ": " }
                            (related_doc_cell(&item.label, &item.url))
                        }
                    }
                }
                p class="text-sm text-base-content/70 my-2" { "This cannot be undone." }
                (form(
                    &CsrfToken::current(),
                    FormOpts {
                        attrs: form_hx_post_route::<IssuedWorkOrderDeleteModalKey, _>(
                            IssuedWorkOrderDeletePostRouteTag::new(self.id),
                        ),
                        form_error: self.error.as_deref(),
                        actions: html! {
                            (button_submit(ButtonSubmit {
                                label: "Confirm Delete",
                                classes: "btn-error",
                                ..Default::default()
                            }))
                        },
                        ..Default::default()
                    },
                ))
            }
        };
        modal_keyed::<IssuedWorkOrderDeleteModalKey>("", body)
    }
}

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
// 9. PREFERENCES
// ==========================================

#[derive(Clone, Generic)]
pub struct WorkOrdersPreferencesPage {
    pub quotation_number_format: String,
    pub draft_work_order_pdf_template: String,
    pub quotation_pdf_template: String,
    pub default_material_tax_items: Vec<ManyToManyItem>,
    pub default_machine_tax_items: Vec<ManyToManyItem>,
    pub error: String,
}

fn pdf_template_editor(
    title: &str,
    field_id: &str,
    value: &str,
    preview_post_url: &str,
    default_hint: &str,
    rows: u32,
) -> Markup {
    html! {
        div class="form-control mb-8" {
            (code_editor_input(CodeEditorInput {
                label: title,
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
        let ctx = FormCtx::form::<WorkOrdersPreferencesForm>(CsrfToken::current())
            .m2m(
                WorkOrdersPreferencesFormField::DefaultMaterialTaxes,
                &self.default_material_tax_items,
            )
            .m2m(
                WorkOrdersPreferencesFormField::DefaultMachineTaxes,
                &self.default_machine_tax_items,
            );
        let tax_fields = html! {
            @for spec in WorkOrdersPreferencesForm::field_specs() {
                @if spec.name == WorkOrdersPreferencesFormField::SectionTaxes.html_name()
                    || spec.name == WorkOrdersPreferencesFormField::DefaultMaterialTaxes.html_name()
                    || spec.name == WorkOrdersPreferencesFormField::DefaultMachineTaxes.html_name()
                {
                    @let field = FieldRender {
                        name: spec.name,
                        label: ctx.label_of(spec),
                        value: ctx.value_of(spec.name),
                        required: spec.required,
                        spec,
                    };
                    ((spec.render)(&ctx, &field))
                }
            }
        };
        form(
            &CsrfToken::current(),
            lariv_rs::components::FormOpts {
                attrs: lariv_rs::components::form_hx_post_main_url(
                    &WorkOrdersPrefsPostRouteTag.url(),
                ),
                title: "KDS Quotations Preferences",
                subtitle: "Configure quotation numbering, default line taxes, and the PDF templates used for draft work orders and quotations. Templates are Jinja2 (Minijinja) that render Typst source; the result is compiled to PDF.",
                form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                inputs: html! {
                    (label_hint(
                        "Quotation number format",
                        Some(crate::work_orders::quotation_number::QUOTATION_NUMBER_FORMAT_HINT),
                        html! {
                            input type="text"
                                name="quotation_number_format"
                                class="input input-bordered w-full"
                                value=(&self.quotation_number_format)
                                placeholder=(crate::work_orders::quotation_number::DEFAULT_QUOTATION_NUMBER_FORMAT) {}
                        },
                    ))
                    (tax_fields)
                    (pdf_template_editor(
                        "Draft Work Order PDF Template",
                        "draft_work_order_pdf_template",
                        &self.draft_work_order_pdf_template,
                        &WorkOrderPdfPreviewPostRouteTag.url(),
                        crate::work_orders::pdf_templates::DEFAULT_DRAFT_WORK_ORDER_PDF_TEMPLATE,
                        18,
                    ))
                    (pdf_template_editor(
                        "Quotation PDF Template",
                        "quotation_pdf_template",
                        &self.quotation_pdf_template,
                        &InvoicePdfPreviewPostRouteTag.url(),
                        crate::work_orders::pdf_templates::DEFAULT_QUOTATION_PDF_TEMPLATE,
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
        scaffold_pane(
            wo_menu("preferences"),
            work_orders_prefs_crumbs(),
            self.body(),
        )
    }
    fn render_main(&self) -> lariv_rs::components::MainContentHtml {
        scaffold_main(work_orders_prefs_crumbs(), self.body())
    }
}

impl RenderTemplate for WorkOrdersPreferencesPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "KDS Quotations Preferences",
            chrome,
            wo_menu("preferences"),
            work_orders_prefs_crumbs(),
            self.body(),
        )
    }
}
