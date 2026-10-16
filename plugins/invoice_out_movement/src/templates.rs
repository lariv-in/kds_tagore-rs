//! Picker table for inventory out movements.

use maud::{Markup, html};

use lariv_core::components::table::table_grid_content;
use lariv_core::components::{
    DataTable, DataTableDisplay, FieldText, FilterClauseView, ListFilterBuilder, ObjectList,
    PaginationPage, SwapKey, TableButtonFilter, TableColumnHeader, TableListContent,
    TablePagination, TableRow, data_table, field_text, form_hx_get_picker_route,
    list_filter_builder, pagination_pages, row_attr_select, table_button_filter,
    table_list_content, table_pagination_picker,
};
use lariv_core::list_filters::FilterField;
use lariv_core::picker::RenderPickerSelect;

use crate::keys::{MovementOutSelectModalKey, MovementOutSelectTableKey};
use crate::routes::MovementOutFkSelectRouteTag;

pub struct MovementOutSelectPage {
    pub rows: ObjectList<(i64, String, String)>,
    pub target_input: String,
    pub path_and_query: String,
    pub page_size: u32,
}

impl RenderPickerSelect<MovementOutSelectTableKey, MovementOutSelectModalKey>
    for MovementOutSelectPage
{
    fn render_table(&self) -> Markup {
        let headers = [
            TableColumnHeader {
                key: "Number",
                label: "Number",
                sort_url: None,
                push_url: false,
            },
            TableColumnHeader {
                key: "Datetime",
                label: "Date & time",
                sort_url: None,
                push_url: false,
            },
        ];
        let rows: Vec<TableRow> = self
            .rows
            .items
            .iter()
            .map(|(id, label, when)| TableRow {
                attrs: row_attr_select(&self.target_input, &id.to_string(), label),
                cells: vec![
                    field_text(FieldText {
                        value: label,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: when,
                        classes: "",
                    }),
                ],
                key: String::new(),
                children: Vec::new(),
            })
            .collect();
        let target_input = self.target_input.clone();
        let fields: &[FilterField] = &[];
        let filter_rows: &[FilterClauseView] = &[];
        let filter = table_button_filter(TableButtonFilter {
            panel: list_filter_builder(ListFilterBuilder {
                fields,
                rows: filter_rows,
                page_size: self.page_size,
                sort: "",
                attrs: form_hx_get_picker_route::<
                    MovementOutSelectTableKey,
                    MovementOutSelectModalKey,
                    MovementOutFkSelectRouteTag,
                >(MovementOutFkSelectRouteTag)
                .set("hx-push-url", "false"),
                apply_label: "Apply",
                extra: html! {
                    input type="hidden" name="target_input" value=(target_input) {}
                },
            }),
            ..Default::default()
        });
        data_table_with_filter_errors::<MovementOutSelectTableKey>(
            "Select out movement",
            filter,
            &headers,
            &rows,
            render_picker_pagination::<MovementOutSelectModalKey>(
                &self.path_and_query,
                self.rows.number,
                self.rows.num_pages,
            ),
            &self.path_and_query,
        )
    }
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

fn data_table_with_filter_errors<K: SwapKey>(
    title: &str,
    actions: Markup,
    headers: &[TableColumnHeader<'_>],
    rows: &[TableRow],
    pagination: Markup,
    refresh_url: &str,
) -> Markup {
    let list = html! {
        (table_list_content(TableListContent {
            headers,
            rows,
            hx_target: K::SELECTOR,
        }))
    };
    let grid = html! {
        (table_grid_content(headers, rows))
    };
    let column_keys: Vec<&str> = headers
        .iter()
        .map(|header| header.key)
        .filter(|key| !key.is_empty())
        .collect();
    data_table(DataTable {
        uid: K::ID,
        title,
        classes: "w-full",
        actions,
        displays: vec![
            DataTableDisplay {
                name: "Grid".into(),
                html: grid,
            },
            DataTableDisplay {
                name: "List".into(),
                html: list,
            },
        ],
        pagination,
        refresh_url,
        column_keys: &column_keys,
        ..Default::default()
    })
}
