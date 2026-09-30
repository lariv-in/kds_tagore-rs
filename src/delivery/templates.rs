use frunk::Generic;
use lariv_rs::{
    components::{
        ButtonModalForm, ButtonSubmit, CodeEditorInput, DeleteConfirmation, DetailHeader,
        FieldText, FormOpts, LayoutMain, LayoutSidebar, ShellChrome, ShellScaffold, SlotCapability,
        SlotRegistrar, SwapKey, TableColumnHeader, TableRow, button_modal_form, button_modal_route,
        button_submit, code_editor_input, container_column, container_row, data_table_list_refresh,
        delete_confirmation, detail, detail_header, field_text, form, label, layout_main,
        layout_sidebar, modal_keyed, row_attr_navigate_route, shell_scaffold, table_create_button,
    },
    html_form::{CsrfToken, FieldRender, FormCtx, FormFieldKey, HtmlForm},
    http::ProvideRequestCaps,
    template::{RenderAppPane, RenderTemplate, TemplateCapability, TemplateOf, TemplateRegistrar},
    web::modal_edit_post_url,
};
use maud::{Markup, html};

use super::crumbs::*;
use super::entities::{delivery_challan, delivery_challan_line};
use super::forms::{
    DeliveryChallanForm, DeliveryChallanFormField, DeliveryPreferencesForm,
    DeliveryPreferencesFormField, lines_form_hx_post,
};
use super::keys::*;
use super::pdf_templates::DEFAULT_DELIVERY_CHALLAN_PDF_TEMPLATE;
use super::routes::*;

lariv_rs::define_register_items! {
    plugin: super::DeliveryTag;
    capability: TemplateCapability;
    trait: TemplateRegistrar;
    method: register_templates;
    wrapper: TemplateOf;
    bounds: [Clone, ProvideRequestCaps, Send, Sync];
    hook: Hook;
    items: [
        ChallanListPageIdx: ChallanListPageTag => ChallanListPage,
        ChallanDetailPageIdx: ChallanDetailPageTag => ChallanDetailPage,
        ChallanCreateModalPageIdx: ChallanCreateModalPageTag => ChallanCreateModalPage,
        ChallanEditModalPageIdx: ChallanEditModalPageTag => ChallanEditModalPage,
        ConfirmDeleteModalPageIdx: ConfirmDeleteModalPageTag => ConfirmDeleteModalPage,
        DeliveryPreferencesPageIdx: DeliveryPreferencesPageTag => DeliveryPreferencesPage,
    ]
}

lariv_rs::define_register_items! {
    plugin: super::DeliveryTag;
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

fn display_or_dash(value: Option<&str>) -> String {
    value
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("—")
        .to_string()
}

#[derive(Clone, Generic)]
pub struct ChallanListPage {
    pub challans: Vec<delivery_challan::Model>,
    pub customer_names: Vec<String>,
    pub path_and_query: String,
}

impl ChallanListPage {
    pub fn render_table(&self) -> Markup {
        let headers = [
            TableColumnHeader {
                key: "ChallanNumber",
                label: "Challan No.",
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
        ];
        let date_labels: Vec<String> = self
            .challans
            .iter()
            .map(|row| row.date.format(lariv_rs::datetime::DATE_FMT).to_string())
            .collect();
        let rows: Vec<TableRow> = self
            .challans
            .iter()
            .enumerate()
            .map(|(i, row)| TableRow {
                attrs: row_attr_navigate_route(DeliveryChallanDetailRouteTag::new(row.id)),
                cells: vec![
                    field_text(FieldText {
                        value: &row.challan_number,
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
                ],
            })
            .collect();
        let actions = html! {
            (table_create_button::<DeliveryChallanTableKey, DeliveryChallanCreateModalKey>(
                Some("plus"),
                "btn-square btn-outline btn-sm",
            ))
        };
        data_table_list_refresh::<DeliveryChallanTableKey>(
            "Delivery Challans",
            actions,
            &headers,
            &rows,
            html! {},
            &self.path_and_query,
        )
    }
}

impl RenderAppPane for ChallanListPage {
    fn render_pane(&self) -> lariv_rs::components::AppLayoutHtml {
        scaffold_pane(
            delivery_menu("challans"),
            challan_list_crumbs(),
            self.render_table(),
        )
    }
    fn render_main(&self) -> lariv_rs::components::MainContentHtml {
        scaffold_main(challan_list_crumbs(), self.render_table())
    }
}

impl RenderTemplate for ChallanListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Delivery Challans — Delivery",
            chrome,
            delivery_menu("challans"),
            challan_list_crumbs(),
            self.render_table(),
        )
    }
}

#[derive(Clone, Generic)]
pub struct ChallanDetailPage {
    pub challan: delivery_challan::Model,
    pub customer_name: String,
    pub lines: Vec<(delivery_challan_line::Model, String)>,
}

impl ChallanDetailPage {
    fn body(&self) -> Markup {
        let edit_url = DeliveryChallanEditGetRouteTag::new(self.challan.id).url();
        let actions = html! {
            (button_modal_form(ButtonModalForm {
                label: "Edit",
                icon_name: Some("pencil"),
                name: "delivery.DeliveryChallanForm",
                href: &edit_url,
                form_post_url: &edit_url,
                modal_uid: DeliveryChallanEditModalKey::ID,
                classes: "btn-outline btn-sm",
                ..Default::default()
            }))
            (button_modal_route(
                DeliveryChallanPdfModalRouteTag::new(self.challan.id),
                "PDF",
                "btn-outline btn-sm",
            ))
        };
        let date_str = self
            .challan
            .date
            .format(lariv_rs::datetime::DATE_FMT)
            .to_string();
        let number = self.challan.challan_number.clone();
        let customer = self.customer_name.clone();
        let vehicle = display_or_dash(self.challan.vehicle_no.as_deref());
        let eway = display_or_dash(self.challan.eway_bill.as_deref());
        let qty_labels: Vec<String> = self
            .lines
            .iter()
            .map(|(line, _)| line.qty_display())
            .collect();
        html! {
            (detail(html! {
                (container_column("", html! {
                    (detail_header(DetailHeader {
                        title: &format!("Delivery Challan {number}"),
                        actions,
                    }))
                    (container_row("gap-6", html! {
                        (label("Challan No.", field_text(FieldText { value: &number, classes: "" })))
                        (label("Date", field_text(FieldText { value: &date_str, classes: "" })))
                        (label("Customer", field_text(FieldText { value: &customer, classes: "" })))
                        (label("Vehicle No.", field_text(FieldText { value: &vehicle, classes: "" })))
                        (label("E-Way Bill", field_text(FieldText { value: &eway, classes: "" })))
                    }))
                    div class="mt-6 min-w-0 w-full" {
                        h4 class="font-bold text-md mb-2" { "Lines" }
                        @if self.lines.is_empty() {
                            p class="text-sm opacity-70" { "No lines on this delivery challan." }
                        } @else {
                            div class="overflow-x-auto min-w-0 w-full" {
                                table class="table table-zebra w-full min-w-max text-sm" {
                                    thead {
                                        tr {
                                            th { "Sr. No." }
                                            th { "Product" }
                                            th { "Qty" }
                                        }
                                    }
                                    tbody {
                                        @for (i, (line, product)) in self.lines.iter().enumerate() {
                                            tr {
                                                td { (line.sr_no) }
                                                td { (product) }
                                                td class="font-mono" { (qty_labels[i]) }
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

impl RenderAppPane for ChallanDetailPage {
    fn render_pane(&self) -> lariv_rs::components::AppLayoutHtml {
        scaffold_pane(
            delivery_menu("challans"),
            challan_crumbs(&self.challan.challan_number, self.challan.id),
            self.body(),
        )
    }
    fn render_main(&self) -> lariv_rs::components::MainContentHtml {
        scaffold_main(
            challan_crumbs(&self.challan.challan_number, self.challan.id),
            self.body(),
        )
    }
}

impl RenderTemplate for ChallanDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            &format!("{} — Delivery", self.challan.challan_number),
            chrome,
            delivery_menu("challans"),
            challan_crumbs(&self.challan.challan_number, self.challan.id),
            self.body(),
        )
    }
}

#[derive(Clone, Generic)]
pub struct ChallanCreateModalPage {
    pub form_name: String,
    pub challan_number: String,
    pub date: String,
    pub vehicle_no: String,
    pub eway_bill: String,
    pub customer_id: Option<i64>,
    pub customer_name: String,
    pub lines_json: String,
    pub error: String,
}

impl RenderTemplate for ChallanCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let cust_id_str = self
            .customer_id
            .map(|id| id.to_string())
            .unwrap_or_default();
        let lines_val = if self.lines_json.is_empty() {
            "[]"
        } else {
            &self.lines_json
        };
        let ctx = FormCtx::form::<DeliveryChallanForm>(CsrfToken::current())
            .value(
                DeliveryChallanFormField::ChallanNumber,
                &self.challan_number,
            )
            .value(DeliveryChallanFormField::Date, &self.date)
            .value(DeliveryChallanFormField::VehicleNo, &self.vehicle_no)
            .value(DeliveryChallanFormField::EwayBill, &self.eway_bill)
            .value(DeliveryChallanFormField::CustomerId, &cust_id_str)
            .display(DeliveryChallanFormField::CustomerId, &self.customer_name)
            .value(DeliveryChallanFormField::Lines, lines_val);
        let modal_classes = format!("!max-w-6xl !w-11/12 {}", self.form_name);
        modal_keyed::<DeliveryChallanCreateModalKey>(
            &modal_classes,
            html! {
                h3 class="font-bold text-lg mb-4" { "New Delivery Challan" }
                @if !self.error.is_empty() {
                    div class="alert alert-error text-sm mb-4 shadow-sm" {
                        span { (self.error) }
                    }
                }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: lines_form_hx_post::<DeliveryChallanCreateModalKey>(
                        &DeliveryChallanCreatePostRouteTag.url(),
                    ),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: html! {
                        (DeliveryChallanForm::render_inputs(&ctx))
                    },
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Create Delivery Challan", ..Default::default() }))
                    },
                    ..Default::default()
                }))
            },
        )
    }
}

#[derive(Clone, Generic)]
pub struct ChallanEditModalPage {
    pub id: i64,
    pub form_name: String,
    pub challan_number: String,
    pub date: String,
    pub vehicle_no: String,
    pub eway_bill: String,
    pub customer_id: i64,
    pub customer_name: String,
    pub lines_json: String,
    pub error: String,
}

impl RenderTemplate for ChallanEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let cust_id_str = self.customer_id.to_string();
        let lines_val = if self.lines_json.is_empty() {
            "[]"
        } else {
            &self.lines_json
        };
        let ctx = FormCtx::form::<DeliveryChallanForm>(CsrfToken::current())
            .value(
                DeliveryChallanFormField::ChallanNumber,
                &self.challan_number,
            )
            .value(DeliveryChallanFormField::Date, &self.date)
            .value(DeliveryChallanFormField::VehicleNo, &self.vehicle_no)
            .value(DeliveryChallanFormField::EwayBill, &self.eway_bill)
            .value(DeliveryChallanFormField::CustomerId, &cust_id_str)
            .display(DeliveryChallanFormField::CustomerId, &self.customer_name)
            .value(DeliveryChallanFormField::Lines, lines_val);
        let delete_url = DeliveryChallanDeleteGetRouteTag::new(self.id).url();
        let modal_classes = format!("!max-w-6xl !w-11/12 {}", self.form_name);
        modal_keyed::<DeliveryChallanEditModalKey>(
            &modal_classes,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit Delivery Challan" }
                @if !self.error.is_empty() {
                    div class="alert alert-error text-sm mb-4 shadow-sm" {
                        span { (self.error) }
                    }
                }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: lines_form_hx_post::<DeliveryChallanEditModalKey>(
                        &modal_edit_post_url(
                            DeliveryChallanEditPostRouteTag::new(self.id),
                            &self.form_name,
                        ),
                    ),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: html! {
                        (DeliveryChallanForm::render_inputs(&ctx))
                    },
                    actions: html! {
                        (button_modal_form(ButtonModalForm {
                            label: "Delete",
                            icon_name: Some("trash"),
                            name: "delivery.ChallanDelete",
                            href: &delete_url,
                            form_post_url: &delete_url,
                            modal_uid: DeliveryChallanDeleteModalKey::ID,
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
pub struct ConfirmDeleteModalPage {
    pub title: String,
    pub message: String,
    pub post_url: String,
    pub error: String,
}

impl RenderTemplate for ConfirmDeleteModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        modal_keyed::<DeliveryChallanDeleteModalKey>(
            "",
            delete_confirmation(DeleteConfirmation {
                title: &self.title,
                message: &self.message,
                attrs: lariv_rs::components::swap::form_hx_post_selector(
                    &self.post_url,
                    &format!("#{}", DeliveryChallanDeleteModalKey::ID),
                ),
                form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                ..Default::default()
            }),
        )
    }
}

#[derive(Clone, Generic)]
pub struct DeliveryPreferencesPage {
    pub challan_number_format: String,
    pub company_name: String,
    pub company_address: String,
    pub company_phone: String,
    pub company_email: String,
    pub company_gstin: String,
    pub terms_and_conditions: String,
    pub logo_vnode_id: String,
    pub logo_vnode_display: String,
    pub signature_vnode_id: String,
    pub signature_vnode_display: String,
    pub delivery_challan_pdf_template: String,
    pub error: String,
}

fn render_pref_field(ctx: &FormCtx<'_>, spec: &lariv_rs::html_form::FieldSpec) -> Markup {
    let field = FieldRender {
        name: spec.name,
        label: ctx.label_of(spec),
        value: ctx.value_of(spec.name),
        required: spec.required,
        spec,
    };
    (spec.render)(ctx, &field)
}

impl DeliveryPreferencesPage {
    fn body(&self) -> Markup {
        let ctx = FormCtx::form::<DeliveryPreferencesForm>(CsrfToken::current())
            .value(
                DeliveryPreferencesFormField::ChallanNumberFormat,
                self.challan_number_format.as_str(),
            )
            .value(
                DeliveryPreferencesFormField::CompanyName,
                self.company_name.as_str(),
            )
            .value(
                DeliveryPreferencesFormField::CompanyAddress,
                self.company_address.as_str(),
            )
            .value(
                DeliveryPreferencesFormField::CompanyPhone,
                self.company_phone.as_str(),
            )
            .value(
                DeliveryPreferencesFormField::CompanyEmail,
                self.company_email.as_str(),
            )
            .value(
                DeliveryPreferencesFormField::CompanyGstin,
                self.company_gstin.as_str(),
            )
            .value(
                DeliveryPreferencesFormField::TermsAndConditions,
                self.terms_and_conditions.as_str(),
            )
            .value(
                DeliveryPreferencesFormField::LogoVnodeId,
                self.logo_vnode_id.as_str(),
            )
            .display(
                DeliveryPreferencesFormField::LogoVnodeId,
                self.logo_vnode_display.as_str(),
            )
            .value(
                DeliveryPreferencesFormField::SignatureVnodeId,
                self.signature_vnode_id.as_str(),
            )
            .display(
                DeliveryPreferencesFormField::SignatureVnodeId,
                self.signature_vnode_display.as_str(),
            )
            .value(
                DeliveryPreferencesFormField::DeliveryChallanPdfTemplate,
                self.delivery_challan_pdf_template.as_str(),
            );
        let template_name = DeliveryPreferencesFormField::DeliveryChallanPdfTemplate.html_name();
        let fields = html! {
            @for spec in DeliveryPreferencesForm::field_specs() {
                @if spec.name != template_name {
                    (render_pref_field(&ctx, spec))
                }
            }
        };
        let template_id = "delivery_challan_pdf_template";
        form(
            &CsrfToken::current(),
            FormOpts {
                attrs: lariv_rs::components::form_hx_post_main_url(
                    &DeliveryPrefsPostRouteTag.url(),
                ),
                title: "Delivery Preferences",
                subtitle: "Company details, challan numbering, and the Typst template used for delivery challan PDFs. The template is Jinja2 (Minijinja) that renders Typst source; the result is compiled to PDF.",
                form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                inputs: html! {
                    (fields)
                    div class="form-control mb-8" {
                        (code_editor_input(CodeEditorInput {
                            label: "Delivery Challan Template (Typst)",
                            name: template_name,
                            value: &self.delivery_challan_pdf_template,
                            id: template_id,
                            language: "typst",
                            rows: 24,
                            max_height: "26rem",
                            required: false,
                            classes: "",
                            attrs: Default::default(),
                            hint: None,
                        }))
                        textarea id=(format!("{template_id}-default")) hidden readonly { (DEFAULT_DELIVERY_CHALLAN_PDF_TEMPLATE) }
                        div class="flex justify-end gap-2 mt-2" {
                            button type="button" class="btn btn-ghost btn-sm"
                                onclick=(format!(
                                    "if (confirm('This will overwrite the template with the default example template. Continue?')) {{ const ta = document.getElementById('{template_id}'); const def = document.getElementById('{template_id}-default'); if (!ta || !def) return; ta.value = def.value; const root = ta.closest('[data-code-editor-root]'); if (root) {{ root.dispatchEvent(new CustomEvent('code-editor:set', {{ detail: {{ value: def.value }} }})); }} else {{ ta.dispatchEvent(new Event('change', {{ bubbles: true }})); }} }}"
                                )) {
                                "Use default template"
                            }
                        }
                    }
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

impl RenderAppPane for DeliveryPreferencesPage {
    fn render_pane(&self) -> lariv_rs::components::AppLayoutHtml {
        scaffold_pane(delivery_menu("preferences"), prefs_crumbs(), self.body())
    }
    fn render_main(&self) -> lariv_rs::components::MainContentHtml {
        scaffold_main(prefs_crumbs(), self.body())
    }
}

impl RenderTemplate for DeliveryPreferencesPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Preferences — Delivery",
            chrome,
            delivery_menu("preferences"),
            prefs_crumbs(),
            self.body(),
        )
    }
}
