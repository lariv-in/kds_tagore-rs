use frunk::Generic;
use lariv_rs::{
    components::{
        ButtonSubmit, Crumb, FormOpts, LayoutMain, LayoutSidebar, ShellChrome, ShellScaffold,
        SlotCapability, SlotRegistrar, breadcrumbs, button_submit, form, form_hx_post_main,
        form_post_download_route, layout_main, layout_sidebar, shell_scaffold,
    },
    html_form::{CsrfToken, FormCtx, HtmlForm},
    http::ProvideRequestCaps,
    plugins::crm::{routes::LeadDefaultRouteTag, templates::crm_menu},
    template::{RenderAppPane, RenderTemplate, TemplateCapability, TemplateOf, TemplateRegistrar},
};
use maud::{Markup, html};

use super::forms::ImportForm;
use super::import::ImportReport;
use super::routes::{MarketingSheetExportRouteTag, MarketingSheetImportRouteTag};
use super::xlsx::{SheetRow, discussion_header};

lariv_rs::define_register_items! {
    plugin: MarketingSheetTag;
    capability: TemplateCapability;
    trait: TemplateRegistrar;
    method: register_templates;
    wrapper: TemplateOf;
    bounds: [Clone, ProvideRequestCaps, Send, Sync];
    hook: Hook;
    items: [
        MarketingSheetPageIdx: MarketingSheetPageTag => MarketingSheetPage,
    ]
}

lariv_rs::define_register_items! {
    plugin: MarketingSheetTag;
    capability: SlotCapability;
    trait: SlotRegistrar;
    method: register_slots;
    bounds: [];
    items: [];
    hook: SlotsHook;
}

fn sheet_crumbs() -> Markup {
    breadcrumbs(&[
        Crumb {
            label: "Leads",
            href: Some(&LeadDefaultRouteTag.url()),
        },
        Crumb {
            label: "Marketing Sheet",
            href: None,
        },
    ])
}

#[derive(Generic)]
pub struct MarketingSheetPage {
    pub error: String,
    pub result: Option<ImportReport>,
    pub rows: Vec<SheetRow>,
}

impl MarketingSheetPage {
    fn body(&self) -> Markup {
        let discussion_dates = {
            let mut dates: Vec<_> = self
                .rows
                .iter()
                .flat_map(|r| r.discussions.iter().map(|(d, _)| *d))
                .collect();
            dates.sort_unstable();
            dates.dedup();
            dates
        };
        html! {
            div class="container max-w-6xl mx-auto" {
                h1 class="text-2xl font-bold mb-2" { "Marketing Sheet" }
                p class="text-sm text-base-content/70 mb-4" {
                    "Import and export CRM leads in the KDS & Tagore Marketing Sheet format. "
                    "Lead Name maps to the salesperson, Current Status to Active / Completed / Failed, "
                    "and dated discussion columns to lead updates."
                }
                @if let Some(report) = &self.result {
                    (report_body(report))
                }
                div class="flex flex-col gap-6 md:flex-row md:items-start" {
                    div class="flex-1 card bg-base-100 border border-base-300" {
                        div class="card-body" {
                            h2 class="card-title text-lg" { "Import" }
                            (form(&CsrfToken::current(), FormOpts {
                                attrs: form_hx_post_main(MarketingSheetImportRouteTag)
                                    .set("hx-encoding", "multipart/form-data"),
                                enctype: Some("multipart/form-data"),
                                form_error: if self.error.is_empty() {
                                    None
                                } else {
                                    Some(self.error.as_str())
                                },
                                inputs: ImportForm::render_inputs(&FormCtx::form::<ImportForm>(CsrfToken::current())),
                                actions: html! {
                                    div class="flex gap-2 mt-4" {
                                        (button_submit(ButtonSubmit {
                                            label: "Import XLSX",
                                            ..Default::default()
                                        }))
                                    }
                                },
                                ..Default::default()
                            }))
                        }
                    }
                    div class="card bg-base-100 border border-base-300" {
                        div class="card-body" {
                            h2 class="card-title text-lg" { "Export" }
                            p class="text-sm text-base-content/70" {
                                "Download the current CRM leads as a marketing sheet."
                            }
                            (form(&CsrfToken::current(), FormOpts {
                                attrs: form_post_download_route(MarketingSheetExportRouteTag),
                                actions: html! {
                                    div class="flex gap-2 mt-4" {
                                        (button_submit(ButtonSubmit {
                                            label: "Download XLSX",
                                            ..Default::default()
                                        }))
                                    }
                                },
                                ..Default::default()
                            }))
                        }
                    }
                }
                div class="mt-8" {
                    h2 class="text-lg font-semibold mb-2" { "Current leads" }
                    @if self.rows.is_empty() {
                        p class="text-sm text-base-content/70" {
                            "No CRM leads yet. Import a sheet or add leads in CRM."
                        }
                    } @else {
                        div class="overflow-x-auto border border-base-300 rounded-box" {
                            table class="table table-sm" {
                                thead {
                                    tr {
                                        th { "S no" }
                                        th { "Lead Name" }
                                        th { "Contact No" }
                                        th { "Customer" }
                                        th { "Company Name" }
                                        th { "Location" }
                                        th { "Status" }
                                        th { "Order date" }
                                        th { "Order Type" }
                                        @for date in &discussion_dates {
                                            th { (discussion_header(*date)) }
                                        }
                                    }
                                }
                                tbody {
                                    @for row in &self.rows {
                                        tr {
                                            td { (row.serial_no.map(|n| n.to_string()).unwrap_or_default()) }
                                            td { (row.lead_name) }
                                            td { (row.contact_no) }
                                            td { (row.customer) }
                                            td { (row.company_name) }
                                            td { (row.expected_sales_location) }
                                            td { (row.status.as_sheet_label()) }
                                            td {
                                                (row.order_expected_date
                                                    .map(lariv_rs::datetime::format_date)
                                                    .unwrap_or_default())
                                            }
                                            td { (row.order_type) }
                                            @for date in &discussion_dates {
                                                td {
                                                    (row.discussions
                                                        .iter()
                                                        .find(|(d, _)| d == date)
                                                        .map(|(_, t)| t.as_str())
                                                        .unwrap_or(""))
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn report_body(report: &ImportReport) -> Markup {
    html! {
        @let alert_class = if report.warnings.is_empty() {
            "alert alert-success mb-4"
        } else {
            "alert alert-warning mb-4"
        };
        div class=(alert_class) {
            div {
                p class="font-semibold" { "Import complete" }
                p class="text-sm" {
                    (report.created) " created, " (report.updated) " updated."
                }
                @if !report.warnings.is_empty() {
                    ul class="list-disc ml-5 mt-2 text-sm" {
                        @for warning in &report.warnings {
                            li { (warning) }
                        }
                    }
                }
            }
        }
    }
}

impl RenderAppPane for MarketingSheetPage {
    fn render_pane(&self) -> lariv_rs::components::AppLayoutHtml {
        layout_sidebar(LayoutSidebar {
            sidebar: crm_menu("marketing"),
            breadcrumbs: sheet_crumbs(),
            content: self.body(),
        })
    }
    fn render_main(&self) -> lariv_rs::components::MainContentHtml {
        layout_main(LayoutMain {
            breadcrumbs: sheet_crumbs(),
            content: self.body(),
        })
    }
}

impl RenderTemplate for MarketingSheetPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        shell_scaffold(ShellScaffold {
            title: "Marketing Sheet — KDS Tagore",
            registry_head: chrome.head.clone(),
            topbar_items: chrome.topbar_items.clone(),
            right_sidebar: chrome.right_sidebar.clone(),
            sidebar: crm_menu("marketing"),
            breadcrumbs: sheet_crumbs(),
            body: self.body(),
            ..Default::default()
        })
    }
}
