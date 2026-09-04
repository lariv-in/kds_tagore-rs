use frunk::Generic;
use lariv_rs::{
    components::{
        ButtonClear, ButtonLink, ButtonModalForm, ButtonSubmit, DeleteConfirmation, DetailHeader,
        FieldDuration, FieldManyToMany, FieldText, FormOpts, HTMX_SWAP_BODY_MODAL,
        HTMX_TARGET_BODY_MODAL, HtmlAttrs, LayoutMain, LayoutSidebar, ManyToManyItem, ObjectList,
        PaginationPage, ShellChrome, ShellScaffold, SidebarMenu, SidebarMenuItem, SlotCapability,
        SlotRegistrar, SwapKey, TableButtonFilter, TableColumnHeader, TablePagination, TableRow,
        button_clear, button_link, button_modal_form, button_submit, column_sort_url,
        container_column, container_error, container_row, data_table_list_refresh,
        delete_confirmation, detail, detail_header, field_duration, field_many_to_many, field_text,
        form, form_hx_get_picker_route, form_hx_get_route, form_hx_post_selector, form_hx_post_url,
        icon, label, layout_main, layout_sidebar, modal, modal_keyed, pagination_pages,
        row_attr_navigate, row_attr_navigate_route, row_attr_select, row_attr_select_multi,
        shell_scaffold, sidebar_menu, sidebar_menu_item_pane, sort_indicator, table_button_filter,
        table_create_button, table_pagination, table_pagination_picker,
    },
    html_form::{FormCtx, HtmlForm},
    http::{ProvideRequestCaps, RouteQueryBuilder, RouteUrl},
    picker::{RenderPickerSelect, picker_create_button},
    plugins::filesystem::routes::VNodeDetailRouteTag,
    template::{RenderAppPane, RenderTemplate, TemplateCapability, TemplateOf, TemplateRegistrar},
    web::{modal_create_post_query, modal_edit_post_url},
};
use maud::{Markup, PreEscaped, html};

use super::crumbs::{
    completed_job_crumbs, job_crumbs, jobs_list_crumbs, machine_crumbs, machines_list_crumbs,
};
use super::forms::{
    JobFilterForm, JobFilterFormField, JobForm, JobFormField, MachineFilterForm,
    MachineFilterFormField, MachineForm, MachineFormField,
};
use super::keys::{
    CompletedJobDeleteModalKey, JobCreateModalKey, JobDeleteModalKey, JobDuplicatedModalKey,
    JobEditModalKey, JobHubTableKey, MachineCreateModalKey, MachineDeleteModalKey,
    MachineEditModalKey, MachineJobsTableKey, MachineSelectModalKey, MachineSelectTableKey,
    MachineTableKey,
};
use super::routes::{
    CompletedJobBulkDeleteGetRouteTag, CompletedJobBulkNewJobPostRouteTag,
    CompletedJobDeleteGetRouteTag, CompletedJobNewJobPostRouteTag, JobBulkDeleteGetRouteTag,
    JobBulkDuplicatePostRouteTag,     JobCreatePostRouteTag, JobDefaultRouteTag, JobDeleteGetRouteTag,
    JobDetailRouteTag, JobDuplicatePostRouteTag, JobEditGetRouteTag, JobEditPostRouteTag,
    JobMoveDownPostRouteTag, JobMoveUpPostRouteTag,
    MachineCreatePostRouteTag, MachineDefaultRouteTag, MachineDeleteGetRouteTag,
    MachineDetailRouteTag, MachineEditGetRouteTag, MachineEditPostRouteTag, MachineFkSelectRouteTag,
};

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

fn button_post_open_modal(route: impl RouteUrl, label: &str, classes: &str) -> Markup {
    let action = route.path();
    let form_attrs = HtmlAttrs::new()
        .set("method", "POST")
        .set("action", &action)
        .set("hx-post", &action)
        .set("hx-target", HTMX_TARGET_BODY_MODAL)
        .set("hx-swap", HTMX_SWAP_BODY_MODAL)
        .set("hx-push-url", "false");
    html! {
        (PreEscaped(format!(
            r##"<form{} @click.stop="">"##,
            form_attrs.as_string()
        )))
        button type="submit" class=(format!("btn {}", classes)) { (label) }
        (PreEscaped("</form>"))
    }
}

fn scaffold_main(crumbs: Markup, body: Markup) -> lariv_rs::components::MainContentHtml {
    layout_main(LayoutMain {
        breadcrumbs: crumbs,
        content: body,
    })
}

fn ms_menu(active: &str) -> Markup {
    sidebar_menu(SidebarMenu {
        title: "Machinery Schedule",
        children: html! {
            (sidebar_menu_item_pane(SidebarMenuItem {
                title: "Jobs",
                url: &JobDefaultRouteTag.url(),
                active: active == "jobs",
                ..Default::default()
            }))
            (sidebar_menu_item_pane(SidebarMenuItem {
                title: "Machines",
                url: &MachineDefaultRouteTag.url(),
                active: active == "machines",
                ..Default::default()
            }))
        },
    })
}

fn tab_href(tab: &str) -> String {
    RouteQueryBuilder::new(JobDefaultRouteTag)
        .query("tab", tab)
        .build()
}

fn hub_selection_root_js() -> &'static str {
    "Alpine.$data($el.closest('[data-ms-job-hub-selection]'))"
}

fn bulk_menu_item(sel: &str, label: &str, classes: &str, on_click: &str) -> String {
    format!(
        r#"<button type="button" class="btn {classes} btn-sm justify-start w-full" x-bind:class="{sel}.selectedIds().length >= 1 ? '' : 'btn-disabled pointer-events-none opacity-50'" @click="{sel}.{on_click}($el); $el.closest('details')?.removeAttribute('open')">{label}</button>"#
    )
}

fn hub_selection_x_data(tab: &str) -> String {
    let delete_get = if tab == "completed" {
        CompletedJobBulkDeleteGetRouteTag.url()
    } else {
        JobBulkDeleteGetRouteTag.url()
    };
    format!(
        r#"{{
            selected: {{}},
            toggle(id) {{
                const k = String(id);
                if (this.selected[k]) delete this.selected[k];
                else this.selected[k] = true;
            }},
            setVisible(ids, on) {{
                for (const id of ids) {{
                    const k = String(id);
                    if (on) this.selected[k] = true;
                    else delete this.selected[k];
                }}
            }},
            allVisibleSelected(ids) {{
                return ids.length > 0 && ids.every(id => !!this.selected[String(id)]);
            }},
            someVisibleSelected(ids) {{
                return ids.some(id => !!this.selected[String(id)]);
            }},
            selectedIds() {{
                return Object.keys(this.selected).filter(k => this.selected[k]);
            }},
            withIds(base) {{
                const ids = this.selectedIds();
                if (ids.length < 1) return '#';
                const join = base.includes('?') ? '&' : '?';
                return base + join + 'ids=' + ids.join(',');
            }},
            requestBulkDelete(el) {{
                const href = this.withIds('{delete_get}');
                if (href === '#' || typeof htmx === 'undefined') return;
                htmx.ajax('GET', href, {{ target: 'body', swap: 'beforeend', source: el }});
            }},
            requestBulkDuplicate(el) {{
                const href = this.withIds('{duplicate_post}');
                if (href === '#' || typeof htmx === 'undefined') return;
                if (!confirm('Create copies of the selected jobs?')) return;
                htmx.ajax('POST', href, {{
                    target: '#app-layout',
                    select: '#app-layout',
                    swap: 'outerHTML',
                    push: true,
                    source: el,
                }});
            }},
            requestBulkNewJob(el) {{
                const href = this.withIds('{new_job_post}');
                if (href === '#' || typeof htmx === 'undefined') return;
                if (!confirm('Create copies of the selected jobs?')) return;
                htmx.ajax('POST', href, {{
                    target: '#app-layout',
                    select: '#app-layout',
                    swap: 'outerHTML',
                    push: true,
                    source: el,
                }});
            }}
        }}"#,
        delete_get = delete_get,
        duplicate_post = JobBulkDuplicatePostRouteTag.url(),
        new_job_post = CompletedJobBulkNewJobPostRouteTag.url()
    )
}

fn tab_nav_link(href: &str, active: bool, label: &str) -> Markup {
    use lariv_rs::components::attrs::escape_attr;
    use maud::PreEscaped;

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

fn render_picker_pagination<M: SwapKey>(
    path_and_query: &str,
    number: u32,
    num_pages: u32,
) -> Markup {
    let owned = pagination_pages(path_and_query, number, num_pages, false);
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
    table_pagination_picker(TablePagination {
        pages: &pages,
        hx_target: M::SELECTOR,
    })
}

fn related_items<'a>(
    items: &'a [(i64, String)],
    hrefs: &'a [String],
) -> Vec<(&'a str, Option<&'a str>)> {
    items
        .iter()
        .zip(hrefs.iter())
        .map(|((_, name), href)| (name.as_str(), Some(href.as_str())))
        .collect()
}

fn job_form_ctx<'a>(
    name: &'a str,
    machines: &'a [ManyToManyItem],
    duration: &'a str,
    files: &'a [ManyToManyItem],
    order: &'a str,
    remarks: &'a str,
    progress: &'a str,
    x_data: &'a str,
) -> FormCtx<'a> {
    FormCtx::form::<JobForm>()
        .value(JobFormField::Name, name)
        .m2m(JobFormField::Machines, machines)
        .value(JobFormField::Duration, duration)
        .m2m(JobFormField::Files, files)
        .value(JobFormField::Order, order)
        .value(JobFormField::Remarks, remarks)
        .value(JobFormField::Progress, progress)
        .x_data(x_data)
        .into()
}

fn query_suffix(path_and_query: &str) -> String {
    match path_and_query.split_once('?') {
        Some((_, query)) if !query.is_empty() => format!("?{query}"),
        _ => String::new(),
    }
}

fn order_move_buttons(id: i64, can_up: bool, can_down: bool, path_and_query: &str) -> Markup {
    let suffix = query_suffix(path_and_query);
    let up = format!("{}{}", JobMoveUpPostRouteTag::new(id).url(), suffix);
    let down = format!("{}{}", JobMoveDownPostRouteTag::new(id).url(), suffix);
    let up_class = if can_up {
        "btn btn-ghost btn-xs btn-square"
    } else {
        "btn btn-ghost btn-xs btn-square btn-disabled"
    };
    let down_class = if can_down {
        "btn btn-ghost btn-xs btn-square"
    } else {
        "btn btn-ghost btn-xs btn-square btn-disabled"
    };
    html! {
        (PreEscaped(r#"<div class="flex items-center gap-1" @click.stop="">"#))
        (PreEscaped(format!(
            r#"<button type="button" class="{up_class}" title="Move up" hx-post="{up}" hx-target="closest .data-table-container" hx-swap="outerMorph" hx-push-url="false"{disabled}>"#,
            up = lariv_rs::components::attrs::escape_attr(&up),
            disabled = if can_up { "" } else { " disabled" },
        )))
        (icon("chevron-up", ""))
        (PreEscaped("</button>"))
        (PreEscaped(format!(
            r#"<button type="button" class="{down_class}" title="Move down" hx-post="{down}" hx-target="closest .data-table-container" hx-swap="outerMorph" hx-push-url="false"{disabled}>"#,
            down = lariv_rs::components::attrs::escape_attr(&down),
            disabled = if can_down { "" } else { " disabled" },
        )))
        (icon("chevron-down", ""))
        (PreEscaped("</button>"))
        (PreEscaped("</div>"))
    }
}

lariv_rs::define_register_items! {
    plugin: MachineryScheduleTag;
    capability: TemplateCapability;
    trait: TemplateRegistrar;
    method: register_templates;
    wrapper: TemplateOf;
    bounds: [Clone, ProvideRequestCaps, Send, Sync];
    hook: Hook;
    items: [
        JobHubIdx: JobHubPageTag => JobHubPage,
        JobDetailIdx: JobDetailPageTag => JobDetailPage,
        JobCreateModalIdx: JobCreateModalPageTag => JobCreateModalPage,
        JobEditModalIdx: JobEditModalPageTag => JobEditModalPage,
        JobDuplicatedModalIdx: JobDuplicatedModalPageTag => JobDuplicatedModalPage,
        CompletedJobDetailIdx: CompletedJobDetailPageTag => CompletedJobDetailPage,
        MachineListIdx: MachineListPageTag => MachineListPage,
        MachineDetailIdx: MachineDetailPageTag => MachineDetailPage,
        MachineCreateModalIdx: MachineCreateModalPageTag => MachineCreateModalPage,
        MachineEditModalIdx: MachineEditModalPageTag => MachineEditModalPage,
        MachineSelectIdx: MachineSelectPageTag => MachineSelectPage,
        ConfirmDeleteIdx: MsConfirmDeletePageTag => ConfirmDeletePage,
        ConfirmBulkDeleteIdx: MsConfirmBulkDeletePageTag => ConfirmBulkDeletePage,
    ]
}

lariv_rs::define_register_items! {
    plugin: MachineryScheduleTag;
    capability: SlotCapability;
    trait: SlotRegistrar;
    method: register_slots;
    bounds: [];
    items: [];
    hook: SlotsHook;
}

#[derive(Clone)]
pub struct JobRow {
    pub id: i64,
    pub name: String,
    pub duration: String,
    pub progress: i16,
    pub order: i64,
    pub machine_count: usize,
    pub extra: String,
    pub detail_href: String,
    pub can_move_up: bool,
    pub can_move_down: bool,
}

#[derive(Generic)]
pub struct JobHubPage {
    pub jobs: ObjectList<JobRow>,
    pub tab: String,
    pub filter_name: String,
    pub sort: String,
    pub path_and_query: String,
    pub can_edit: bool,
}

impl JobHubPage {
    fn tab_link(&self, tab: &str, label: &str) -> Markup {
        tab_nav_link(&tab_href(tab), self.tab == tab, label)
    }

    fn wrap_with_selection(&self, table: Markup) -> Markup {
        html! {
            (PreEscaped(format!(
                r#"<div data-ms-job-hub-selection x-data="{}">"#,
                lariv_rs::components::attrs::escape_attr(&hub_selection_x_data(&self.tab)),
            )))
            (table)
            (PreEscaped("</div>"))
        }
    }

    fn bulk_actions(&self) -> Markup {
        if !self.can_edit {
            return html! {};
        }
        let sel = hub_selection_root_js();
        let mut items = String::new();
        match self.tab.as_str() {
            "completed" => {
                items.push_str(&bulk_menu_item(
                    sel,
                    "Delete",
                    "btn-ghost text-error",
                    "requestBulkDelete",
                ));
                items.push_str(&bulk_menu_item(sel, "New Job", "btn-ghost", "requestBulkNewJob"));
            }
            _ => {
                items.push_str(&bulk_menu_item(
                    sel,
                    "Delete",
                    "btn-ghost text-error",
                    "requestBulkDelete",
                ));
                items.push_str(&bulk_menu_item(
                    sel,
                    "Duplicate",
                    "btn-ghost",
                    "requestBulkDuplicate",
                ));
            }
        }
        html! {
            (PreEscaped(
                r#"<details class="dropdown dropdown-end" @click.outside="$el.removeAttribute('open')">"#,
            ))
            summary class="btn btn-outline btn-sm dropdown-toggle w-32" {
                "Bulk actions"
            }
            div class="card w-56 my-1.5 card-body shadow dropdown-content border border-base-300 rounded-box z-50 bg-base-100 p-2" {
                div class="flex flex-col gap-1" {
                    (PreEscaped(items))
                }
            }
            (PreEscaped("</details>"))
        }
    }

    pub fn render_table(&self) -> Markup {
        let extra_key = if self.tab == "completed" {
            "Completed"
        } else {
            ""
        };
        let show_select = self.can_edit;
        let sel = hub_selection_root_js();
        let name_sort = column_sort_url(&self.path_and_query, "Name", &self.sort);
        let name_label = format!("Name{}", sort_indicator(&self.sort, "Name"));
        let duration_sort = column_sort_url(&self.path_and_query, "Duration", &self.sort);
        let duration_label = format!("Duration{}", sort_indicator(&self.sort, "Duration"));
        let progress_sort = column_sort_url(&self.path_and_query, "Progress", &self.sort);
        let progress_label = format!("Progress{}", sort_indicator(&self.sort, "Progress"));
        let order_sort = column_sort_url(&self.path_and_query, "Order", &self.sort);
        let order_label = format!("Order{}", sort_indicator(&self.sort, "Order"));
        let machines_sort = column_sort_url(&self.path_and_query, "Machines", &self.sort);
        let machines_label = format!("Machines{}", sort_indicator(&self.sort, "Machines"));
        let completed_sort = column_sort_url(&self.path_and_query, "Completed", &self.sort);
        let completed_label = format!("Completed{}", sort_indicator(&self.sort, "Completed"));
        let mut headers = Vec::new();
        if show_select {
            headers.push(TableColumnHeader {
                key: "Select",
                label: "Select",
                sort_url: None,
                push_url: true,
            });
        }
        headers.push(TableColumnHeader {
            key: "Name",
            label: &name_label,
            sort_url: Some(&name_sort),
            push_url: true,
        });
        headers.push(TableColumnHeader {
            key: "Duration",
            label: &duration_label,
            sort_url: Some(&duration_sort),
            push_url: true,
        });
        headers.push(TableColumnHeader {
            key: "Progress",
            label: &progress_label,
            sort_url: Some(&progress_sort),
            push_url: true,
        });
        headers.push(TableColumnHeader {
            key: "Order",
            label: &order_label,
            sort_url: Some(&order_sort),
            push_url: true,
        });
        headers.push(TableColumnHeader {
            key: "Machines",
            label: &machines_label,
            sort_url: Some(&machines_sort),
            push_url: true,
        });
        let show_move = self.can_edit && self.tab != "completed";
        if show_move {
            headers.push(TableColumnHeader {
                key: "Move",
                label: "Move",
                sort_url: None,
                push_url: true,
            });
        }
        if !extra_key.is_empty() {
            headers.push(TableColumnHeader {
                key: extra_key,
                label: &completed_label,
                sort_url: Some(&completed_sort),
                push_url: true,
            });
        }
        let progress_labels: Vec<String> = self
            .jobs
            .items
            .iter()
            .map(|j| format!("{}%", j.progress))
            .collect();
        let order_labels: Vec<String> = self.jobs.items.iter().map(|j| j.order.to_string()).collect();
        let machine_labels: Vec<String> = self
            .jobs
            .items
            .iter()
            .map(|j| j.machine_count.to_string())
            .collect();
        let rows: Vec<TableRow> = self
            .jobs
            .items
            .iter()
            .enumerate()
            .map(|(i, j)| {
                let mut cells = Vec::new();
                if show_select {
                    cells.push(PreEscaped(format!(
                        r#"<label class="flex justify-center" @click.stop=""><input type="checkbox" class="checkbox checkbox-sm" @change="{sel}.toggle({id})" :checked="!!{sel}.selected['{id}']" /></label>"#,
                        sel = sel,
                        id = j.id,
                    )));
                }
                cells.push(field_text(FieldText {
                    value: &j.name,
                    classes: "",
                }));
                cells.push(field_duration(FieldDuration {
                    value: &j.duration,
                    classes: "",
                }));
                cells.push(field_text(FieldText {
                    value: &progress_labels[i],
                    classes: "",
                }));
                cells.push(field_text(FieldText {
                    value: &order_labels[i],
                    classes: "",
                }));
                cells.push(field_text(FieldText {
                    value: &machine_labels[i],
                    classes: "",
                }));
                if show_move {
                    cells.push(order_move_buttons(
                        j.id,
                        j.can_move_up,
                        j.can_move_down,
                        &self.path_and_query,
                    ));
                }
                if !extra_key.is_empty() {
                    cells.push(field_text(FieldText {
                        value: &j.extra,
                        classes: "",
                    }));
                }
                TableRow {
                    attrs: row_attr_navigate(&j.detail_href),
                    cells,
                }
            })
            .collect();
        let mut actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: form(FormOpts {
                    attrs: form_hx_get_route::<JobHubTableKey, JobDefaultRouteTag>(
                        JobDefaultRouteTag,
                    ),
                    inputs: html! {
                        input type="hidden" name="tab" value=(self.tab) {}
                        (JobFilterForm::render_inputs(
                            &FormCtx::form::<JobFilterForm>()
                                .value(JobFilterFormField::Name, &self.filter_name),
                        ))
                    },
                    actions: html! {
                        (container_row("flex gap-2", html! {
                            (button_submit(ButtonSubmit { label: "Apply", ..Default::default() }))
                            (button_clear(ButtonClear { label: "Clear", ..Default::default() }))
                        }))
                    },
                    ..Default::default()
                }),
                ..Default::default()
            }))
        };
        if self.can_edit && self.tab == "jobs" {
            actions = html! {
                (actions)
                (table_create_button::<JobHubTableKey, JobCreateModalKey>(
                    Some("plus"),
                    "btn-square btn-outline btn-sm",
                ))
            };
        }
        if self.can_edit {
            actions = html! {
                (actions)
                (self.bulk_actions())
            };
        }
        data_table_list_refresh::<JobHubTableKey>(
            "Jobs",
            actions,
            &headers,
            &rows,
            render_pagination::<JobHubTableKey>(
                &self.path_and_query,
                self.jobs.number,
                self.jobs.num_pages,
            ),
            &self.path_and_query,
        )
    }

    fn body(&self) -> Markup {
        let table = self.render_table();
        let table = if self.can_edit {
            self.wrap_with_selection(table)
        } else {
            table
        };
        html! {
            div class="tabs tabs-boxed mb-4" {
                (self.tab_link("jobs", "Jobs"))
                (self.tab_link("completed", "Completed"))
            }
            (table)
        }
    }
}

impl RenderAppPane for JobHubPage {
    fn render_pane(&self) -> lariv_rs::components::AppLayoutHtml {
        scaffold_pane(ms_menu("jobs"), jobs_list_crumbs(), self.body())
    }
    fn render_main(&self) -> lariv_rs::components::MainContentHtml {
        scaffold_main(jobs_list_crumbs(), self.body())
    }
}

impl RenderTemplate for JobHubPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Machinery Schedule — Jobs",
            chrome,
            ms_menu("jobs"),
            jobs_list_crumbs(),
            self.body(),
        )
    }
}

fn job_fields(
    duration: &str,
    progress: i16,
    order: i64,
    remarks: &str,
    machines: &[(i64, String)],
    files: &[(i64, String)],
) -> Markup {
    let machine_hrefs: Vec<String> = machines
        .iter()
        .map(|(id, _)| MachineDetailRouteTag::new(*id).url())
        .collect();
    let file_hrefs: Vec<String> = files
        .iter()
        .map(|(id, _)| VNodeDetailRouteTag::new(*id).url())
        .collect();
    let machine_items = related_items(machines, &machine_hrefs);
    let file_items = related_items(files, &file_hrefs);
    let progress_label = format!("{progress}%");
    let order_label = order.to_string();
    html! {
        (label("Duration", field_duration(FieldDuration {
            value: duration,
            classes: "",
        })))
        (label("Progress", field_text(FieldText {
            value: &progress_label,
            classes: "",
        })))
        (label("Order", field_text(FieldText {
            value: &order_label,
            classes: "",
        })))
        (label("Remarks", field_text(FieldText {
            value: remarks,
            classes: "",
        })))
        (label("Machines", field_many_to_many(FieldManyToMany {
            items: &machine_items,
            classes: "",
        })))
        (label("Files", field_many_to_many(FieldManyToMany {
            items: &file_items,
            classes: "",
        })))
    }
}

#[derive(Generic)]
pub struct JobDetailPage {
    pub id: i64,
    pub name: String,
    pub duration: String,
    pub progress: i16,
    pub order: i64,
    pub remarks: String,
    pub machines: Vec<(i64, String)>,
    pub files: Vec<(i64, String)>,
    pub can_edit: bool,
    pub error: String,
}

impl JobDetailPage {
    fn body(&self) -> Markup {
        let actions = if self.can_edit {
            html! {
                (button_post_open_modal(
                    JobDuplicatePostRouteTag::new(self.id),
                    "Duplicate",
                    "btn-outline",
                ))
                (button_modal_form(ButtonModalForm {
                    name: "kds_ms.JobEditForm",
                    href: &JobEditGetRouteTag::new(self.id).url(),
                    form_post_url: &JobEditPostRouteTag::new(self.id).path(),
                    modal_uid: JobEditModalKey::ID,
                    label: "Edit",
                    classes: "btn-outline",
                    ..Default::default()
                }))
            }
        } else {
            html! {}
        };
        html! {
            (detail(html! {
                (container_column("", html! {
                    (detail_header(DetailHeader {
                        title: &self.name,
                        actions,
                    }))
                    @if !self.error.is_empty() {
                        (container_error(Some(self.error.as_str()), html! {}))
                    }
                    (job_fields(
                        &self.duration,
                        self.progress,
                        self.order,
                        &self.remarks,
                        &self.machines,
                        &self.files,
                    ))
                }))
            }))
        }
    }
}

impl RenderAppPane for JobDetailPage {
    fn render_pane(&self) -> lariv_rs::components::AppLayoutHtml {
        scaffold_pane(
            ms_menu("jobs"),
            job_crumbs(&self.name, self.id, None),
            self.body(),
        )
    }
    fn render_main(&self) -> lariv_rs::components::MainContentHtml {
        scaffold_main(job_crumbs(&self.name, self.id, None), self.body())
    }
}

impl RenderTemplate for JobDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Job — Machinery Schedule",
            chrome,
            ms_menu("jobs"),
            job_crumbs(&self.name, self.id, None),
            self.body(),
        )
    }
}

#[derive(Generic)]
pub struct JobDuplicatedModalPage {
    pub job_id: i64,
    pub name: String,
    pub error: String,
}

impl RenderTemplate for JobDuplicatedModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        if !self.error.is_empty() {
            return modal_keyed::<JobDuplicatedModalKey>(
                "!max-w-md",
                html! {
                    h3 class="font-bold text-lg mb-2" { "Could not duplicate job" }
                    p class="text-error" { (self.error) }
                },
            );
        }
        let open_url = JobDetailRouteTag::new(self.job_id).url();
        let close = format!(
            "document.getElementById('{}').remove()",
            JobDuplicatedModalKey::ID
        );
        modal_keyed::<JobDuplicatedModalKey>(
            "!max-w-md",
            html! {
                h3 class="font-bold text-lg mb-4 flex items-center gap-2" {
                    (icon("check-circle", "text-success"))
                    "Job duplicated"
                }
                div class="card bg-base-100 border border-base-content/10 shadow-sm" {
                    div class="card-body gap-1 py-4" {
                        h4 class="card-title text-base" { (self.name) }
                        p class="text-sm opacity-70" { "Copy created with progress 0." }
                    }
                }
                div class="modal-action" {
                    (button_link(ButtonLink {
                        label: "Open job",
                        href: &open_url,
                        classes: "btn-primary",
                        attrs: HtmlAttrs::new().set("onclick", &close),
                        ..Default::default()
                    }))
                }
            },
        )
    }
}

#[derive(Generic)]
pub struct CompletedJobDetailPage {
    pub id: i64,
    pub name: String,
    pub duration: String,
    pub progress: i16,
    pub order: i64,
    pub remarks: String,
    pub completed_at: String,
    pub machines: Vec<(i64, String)>,
    pub files: Vec<(i64, String)>,
    pub can_edit: bool,
}

impl CompletedJobDetailPage {
    fn body(&self) -> Markup {
        let delete_url = CompletedJobDeleteGetRouteTag::new(self.id).url();
        let actions = if self.can_edit {
            html! {
                (button_post_open_modal(
                    CompletedJobNewJobPostRouteTag::new(self.id),
                    "New job",
                    "btn-outline",
                ))
                (button_modal_form(ButtonModalForm {
                    label: "Delete",
                    icon_name: Some("trash"),
                    name: "kds_ms.CompletedJobDeleteForm",
                    href: &delete_url,
                    form_post_url: &delete_url,
                    modal_uid: CompletedJobDeleteModalKey::ID,
                    classes: "btn-error",
                    ..Default::default()
                }))
            }
        } else {
            html! {}
        };
        html! {
            (detail(html! {
                (container_column("", html! {
                    (detail_header(DetailHeader {
                        title: &self.name,
                        actions,
                    }))
                    (label("Completed at", field_text(FieldText { value: &self.completed_at, classes: "" })))
                    (job_fields(
                        &self.duration,
                        self.progress,
                        self.order,
                        &self.remarks,
                        &self.machines,
                        &self.files,
                    ))
                }))
            }))
        }
    }
}

impl RenderAppPane for CompletedJobDetailPage {
    fn render_pane(&self) -> lariv_rs::components::AppLayoutHtml {
        scaffold_pane(
            ms_menu("jobs"),
            completed_job_crumbs(&self.name, self.id, None),
            self.body(),
        )
    }
    fn render_main(&self) -> lariv_rs::components::MainContentHtml {
        scaffold_main(completed_job_crumbs(&self.name, self.id, None), self.body())
    }
}

impl RenderTemplate for CompletedJobDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Completed job — Machinery Schedule",
            chrome,
            ms_menu("jobs"),
            completed_job_crumbs(&self.name, self.id, None),
            self.body(),
        )
    }
}

#[derive(Generic)]
pub struct JobCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub target_input: String,
    pub name: String,
    pub machines: Vec<ManyToManyItem>,
    pub duration: String,
    pub files: Vec<ManyToManyItem>,
    pub order: i64,
    pub remarks: String,
    pub progress: i16,
    pub error: String,
}

impl RenderTemplate for JobCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let order = self.order.to_string();
        let progress = self.progress.to_string();
        let progress_x_data = format!("{{ progress: {} }}", self.progress);
        modal_keyed::<JobCreateModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "New job" }
                (form(FormOpts {
                    attrs: form_hx_post_url::<JobCreateModalKey>(&modal_create_post_query(
                        JobCreatePostRouteTag,
                        &self.form_name,
                        &self.refresh_table,
                        &self.target_input,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: JobForm::render_inputs(&job_form_ctx(
                        &self.name,
                        &self.machines,
                        &self.duration,
                        &self.files,
                        &order,
                        &self.remarks,
                        &progress,
                        &progress_x_data,
                    )),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Create job", ..Default::default() }))
                    },
                    ..Default::default()
                }))
            },
        )
    }
}

#[derive(Generic)]
pub struct JobEditModalPage {
    pub id: i64,
    pub form_name: String,
    pub name: String,
    pub machines: Vec<ManyToManyItem>,
    pub duration: String,
    pub files: Vec<ManyToManyItem>,
    pub order: i64,
    pub remarks: String,
    pub progress: i16,
    pub error: String,
}

impl RenderTemplate for JobEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let delete_url = JobDeleteGetRouteTag::new(self.id).url();
        let order = self.order.to_string();
        let progress = self.progress.to_string();
        let progress_x_data = format!("{{ progress: {} }}", self.progress);
        modal_keyed::<JobEditModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit job" }
                (form(FormOpts {
                    attrs: form_hx_post_url::<JobEditModalKey>(&modal_edit_post_url(
                        JobEditPostRouteTag::new(self.id),
                        &self.form_name,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: JobForm::render_inputs(&job_form_ctx(
                        &self.name,
                        &self.machines,
                        &self.duration,
                        &self.files,
                        &order,
                        &self.remarks,
                        &progress,
                        &progress_x_data,
                    )),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Save", ..Default::default() }))
                        (button_modal_form(ButtonModalForm {
                            label: "Delete",
                            icon_name: Some("trash"),
                            name: "kds_ms.JobDeleteForm",
                            href: &delete_url,
                            form_post_url: &delete_url,
                            modal_uid: JobDeleteModalKey::ID,
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

#[derive(Clone)]
pub struct MachineRow {
    pub id: i64,
    pub name: String,
}

#[derive(Generic)]
pub struct MachineListPage {
    pub machines: ObjectList<MachineRow>,
    pub filter_name: String,
    pub sort: String,
    pub path_and_query: String,
    pub can_edit: bool,
}

impl MachineListPage {
    pub fn render_table(&self) -> Markup {
        let name_sort = column_sort_url(&self.path_and_query, "Name", &self.sort);
        let name_label = format!("Name{}", sort_indicator(&self.sort, "Name"));
        let headers = [TableColumnHeader {
            key: "Name",
            label: &name_label,
            sort_url: Some(&name_sort),
            push_url: true,
        }];
        let rows: Vec<TableRow> = self
            .machines
            .items
            .iter()
            .map(|m| TableRow {
                attrs: row_attr_navigate_route(MachineDetailRouteTag::new(m.id)),
                cells: vec![field_text(FieldText {
                    value: &m.name,
                    classes: "",
                })],
            })
            .collect();
        let mut actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: form(FormOpts {
                    attrs: form_hx_get_route::<MachineTableKey, MachineDefaultRouteTag>(
                        MachineDefaultRouteTag,
                    ),
                    inputs: MachineFilterForm::render_inputs(
                        &FormCtx::form::<MachineFilterForm>()
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
                (table_create_button::<MachineTableKey, MachineCreateModalKey>(
                    Some("plus"),
                    "btn-square btn-outline btn-sm",
                ))
            };
        }
        data_table_list_refresh::<MachineTableKey>(
            "Machines",
            actions,
            &headers,
            &rows,
            render_pagination::<MachineTableKey>(
                &self.path_and_query,
                self.machines.number,
                self.machines.num_pages,
            ),
            &self.path_and_query,
        )
    }
}

impl RenderAppPane for MachineListPage {
    fn render_pane(&self) -> lariv_rs::components::AppLayoutHtml {
        scaffold_pane(
            ms_menu("machines"),
            machines_list_crumbs(),
            self.render_table(),
        )
    }
    fn render_main(&self) -> lariv_rs::components::MainContentHtml {
        scaffold_main(machines_list_crumbs(), self.render_table())
    }
}

impl RenderTemplate for MachineListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Machines — Machinery Schedule",
            chrome,
            ms_menu("machines"),
            machines_list_crumbs(),
            self.render_table(),
        )
    }
}

#[derive(Clone)]
pub struct MachineJobRow {
    pub id: i64,
    pub name: String,
    pub duration: String,
    pub progress: i16,
    pub order: i64,
    pub detail_href: String,
}

#[derive(Generic)]
pub struct MachineDetailPage {
    pub id: i64,
    pub name: String,
    pub can_edit: bool,
    pub jobs: Vec<MachineJobRow>,
    pub free_on: String,
}

impl MachineDetailPage {
    pub fn render_jobs_table(&self) -> Markup {
        let headers = [
            TableColumnHeader {
                key: "Name",
                label: "Name",
                sort_url: None,
                push_url: true,
            },
            TableColumnHeader {
                key: "Duration",
                label: "Duration",
                sort_url: None,
                push_url: true,
            },
            TableColumnHeader {
                key: "Progress",
                label: "Progress",
                sort_url: None,
                push_url: true,
            },
            TableColumnHeader {
                key: "Order",
                label: "Order",
                sort_url: None,
                push_url: true,
            },
        ];
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
        data_table_list_refresh::<MachineJobsTableKey>(
            "Jobs",
            html! {},
            &headers,
            &rows,
            html! {},
            &MachineDetailRouteTag::new(self.id).url(),
        )
    }

    fn body(&self) -> Markup {
        let actions = if self.can_edit {
            html! {
                (button_modal_form(ButtonModalForm {
                    name: "kds_ms.MachineEditForm",
                    href: &MachineEditGetRouteTag::new(self.id).url(),
                    form_post_url: &MachineEditPostRouteTag::new(self.id).path(),
                    modal_uid: MachineEditModalKey::ID,
                    label: "Edit",
                    classes: "btn-outline",
                    ..Default::default()
                }))
            }
        } else {
            html! {}
        };
        html! {
            (detail(html! {
                (container_column("", html! {
                    (detail_header(DetailHeader {
                        title: &self.name,
                        actions,
                    }))
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

impl RenderAppPane for MachineDetailPage {
    fn render_pane(&self) -> lariv_rs::components::AppLayoutHtml {
        scaffold_pane(
            ms_menu("machines"),
            machine_crumbs(&self.name, self.id, None),
            self.body(),
        )
    }
    fn render_main(&self) -> lariv_rs::components::MainContentHtml {
        scaffold_main(machine_crumbs(&self.name, self.id, None), self.body())
    }
}

impl RenderTemplate for MachineDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Machine — Machinery Schedule",
            chrome,
            ms_menu("machines"),
            machine_crumbs(&self.name, self.id, None),
            self.body(),
        )
    }
}

#[derive(Generic)]
pub struct MachineCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub target_input: String,
    pub name: String,
    pub error: String,
}

impl RenderTemplate for MachineCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        modal_keyed::<MachineCreateModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "New machine" }
                (form(FormOpts {
                    attrs: form_hx_post_url::<MachineCreateModalKey>(&modal_create_post_query(
                        MachineCreatePostRouteTag,
                        &self.form_name,
                        &self.refresh_table,
                        &self.target_input,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: MachineForm::render_inputs(
                        &FormCtx::form::<MachineForm>().value(MachineFormField::Name, &self.name),
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
pub struct MachineEditModalPage {
    pub id: i64,
    pub form_name: String,
    pub name: String,
    pub error: String,
}

impl RenderTemplate for MachineEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let delete_url = MachineDeleteGetRouteTag::new(self.id).url();
        modal_keyed::<MachineEditModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit machine" }
                (form(FormOpts {
                    attrs: form_hx_post_url::<MachineEditModalKey>(&modal_edit_post_url(
                        MachineEditPostRouteTag::new(self.id),
                        &self.form_name,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: MachineForm::render_inputs(
                        &FormCtx::form::<MachineForm>().value(MachineFormField::Name, &self.name),
                    ),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Save", ..Default::default() }))
                        (button_modal_form(ButtonModalForm {
                            label: "Delete",
                            icon_name: Some("trash"),
                            name: "kds_ms.MachineDeleteForm",
                            href: &delete_url,
                            form_post_url: &delete_url,
                            modal_uid: MachineDeleteModalKey::ID,
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

#[derive(Generic)]
pub struct MachineSelectPage {
    pub machines: ObjectList<MachineRow>,
    pub filter_name: String,
    pub target_input: String,
    pub sort: String,
    pub path_and_query: String,
    pub can_edit: bool,
    pub multi: bool,
}

impl RenderPickerSelect<MachineSelectTableKey, MachineSelectModalKey> for MachineSelectPage {
    fn render_table(&self) -> Markup {
        let name_sort = column_sort_url(&self.path_and_query, "Name", &self.sort);
        let name_label = format!("Name{}", sort_indicator(&self.sort, "Name"));
        let headers = [TableColumnHeader {
            key: "Name",
            label: &name_label,
            sort_url: Some(&name_sort),
            push_url: false,
        }];
        let rows: Vec<TableRow> = self
            .machines
            .items
            .iter()
            .map(|m| TableRow {
                attrs: if self.multi {
                    row_attr_select_multi(&self.target_input, &m.id.to_string(), &m.name)
                } else {
                    row_attr_select(&self.target_input, &m.id.to_string(), &m.name)
                },
                cells: vec![field_text(FieldText {
                    value: &m.name,
                    classes: "",
                })],
            })
            .collect();
        let mut actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: form(FormOpts {
                    attrs: form_hx_get_picker_route::<
                        MachineSelectTableKey,
                        MachineSelectModalKey,
                        MachineFkSelectRouteTag,
                    >(MachineFkSelectRouteTag)
                    .set("hx-push-url", "false"),
                    inputs: html! {
                        (MachineFilterForm::render_inputs(
                            &FormCtx::form::<MachineFilterForm>()
                                .value(MachineFilterFormField::Name, &self.filter_name),
                        ))
                        input type="hidden" name="target_input" value=(self.target_input) {}
                        @if self.multi {
                            input type="hidden" name="multi" value="1" {}
                        }
                    },
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
                (picker_create_button::<MachineCreateModalKey>(
                    &self.target_input,
                    Some("plus"),
                    "btn-square btn-outline btn-sm",
                ))
            };
        }
        data_table_list_refresh::<MachineSelectTableKey>(
            "Select machine",
            actions,
            &headers,
            &rows,
            render_picker_pagination::<MachineSelectModalKey>(
                &self.path_and_query,
                self.machines.number,
                self.machines.num_pages,
            ),
            &self.path_and_query,
        )
    }
}

impl RenderTemplate for MachineSelectPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        self.render_modal().into_inner()
    }
}

#[derive(Generic)]
pub struct ConfirmDeletePage {
    pub modal_uid: String,
    pub message: String,
    pub form_name: String,
    pub post_url: String,
    pub error: String,
}

impl RenderTemplate for ConfirmDeletePage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let target = format!("#{}", self.modal_uid);
        modal(lariv_rs::components::Modal {
            uid: &self.modal_uid,
            children: delete_confirmation(DeleteConfirmation {
                title: "Confirm Deletion",
                message: &self.message,
                attrs: form_hx_post_selector(&self.post_url, &target),
                form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                ..Default::default()
            }),
            ..Default::default()
        })
    }
}

#[derive(Generic)]
pub struct ConfirmBulkDeletePage {
    pub modal_uid: String,
    pub message: String,
    pub ids: String,
    pub post_url: String,
    pub error: String,
    pub can_submit: bool,
}

impl RenderTemplate for ConfirmBulkDeletePage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let target = format!("#{}", self.modal_uid);
        let form_attrs = form_hx_post_selector(&self.post_url, &target);
        modal(lariv_rs::components::Modal {
            uid: &self.modal_uid,
            children: html! {
                div class="container mx-auto" {
                    h2 class="text-xl font-bold text-error" { "Confirm Deletion" }
                    p class="my-2" { (self.message) }
                    @if !self.error.is_empty() {
                        div class="alert alert-error my-2 text-sm" { (self.error) }
                    }
                    @if self.can_submit {
                        (PreEscaped(format!(
                            r#"<form class="flex flex-col gap-2 my-4"{}>"#,
                            form_attrs.as_string(),
                        )))
                        input type="hidden" name="ids" value=(self.ids);
                        div class="my-2" {
                            (button_submit(ButtonSubmit {
                                label: "Confirm Delete",
                                classes: "btn-error my-2",
                                ..Default::default()
                            }))
                        }
                        (PreEscaped("</form>"))
                    }
                }
            },
            ..Default::default()
        })
    }
}
