use axum::{extract::Query, http::Uri};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};

use lariv_core::components::ObjectList;
use lariv_core::http::Cap;
use lariv_core::picker::respond_picker_select;
use lariv_core::web::{Htmx, QueryPage, QueryPageSize};
use lariv_plugin_inventory::entities::stock_movement_out::{self, Entity as OutEntity};
use lariv_plugin_inventory::scope::page_of;
use lariv_plugin_inventory::state::InventoryState;
use lariv_plugin_users::middleware::RequireAuth;
use lariv_plugin_users::role_authorization::scope_allowed;

use lariv_plugin_finance_invoices::routes::FinanceInvoicesView;

use crate::keys::{MovementOutSelectModalKey, MovementOutSelectTableKey};
use crate::templates::MovementOutSelectPage;

#[derive(Debug, serde::Deserialize, Default)]
pub struct OutSelectQuery {
    #[serde(default)]
    page: QueryPage,
    #[serde(default)]
    page_size: QueryPageSize,
    #[serde(default, rename = "Name")]
    name: Option<String>,
    #[serde(default)]
    target_input: Option<String>,
}

pub async fn select_out(
    Cap(state): Cap<InventoryState>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<OutSelectQuery>,
) -> maud::Markup {
    let mut query = scope_allowed::<FinanceInvoicesView, _>(OutEntity::find());
    if let Some(name) = q
        .name
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
    {
        query = query.filter(stock_movement_out::Column::Number.contains(name));
    }
    query = query.order_by_desc(stock_movement_out::Column::Datetime);
    let (models, page, total) = page_of(query, &state.db, q.page.get(), q.page_size.get()).await;
    let rows = models
        .into_iter()
        .map(|row| {
            let when = ctx.format_datetime(row.datetime).into_string();
            let label = if row.number.trim().is_empty() {
                format!("#{}", row.id)
            } else {
                row.number
            };
            (row.id, label, when)
        })
        .collect();
    let page = MovementOutSelectPage {
        rows: ObjectList::from_page(rows, page, q.page_size.get(), total),
        target_input: q.target_input.unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        page_size: q.page_size.get(),
    };
    respond_picker_select::<MovementOutSelectTableKey, MovementOutSelectModalKey, _>(&htmx, &page)
}

fn path_and_query(uri: &Uri) -> String {
    uri.path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string())
}
