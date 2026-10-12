//! List-filter catalogs for quotations, work orders, and components.

use rust_decimal::Decimal;
use sea_orm::sea_query::Condition;
use sea_orm::{DatabaseBackend, DatabaseConnection, EntityTrait};

use lariv_core::components::FilterClauseView;
use lariv_core::list_filters::{
    CmpOp, FilterClause, FilterField, FilterRowError, FilterWidget, apply_filters, custom,
    date_column, foreign_key_column, number_column, parse_list_filters, text_column,
};
use lariv_core::web::{QueryPage, QueryPageSize};
use lariv_plugin_contacts::entities::company;
use lariv_plugin_contacts::routes::CompanyFkSelectRouteTag;

use crate::entities::{component, draft_work_order, quotation, work_order};
use crate::forms::schema_entries_from_json;

#[derive(Debug, serde::Deserialize, Default)]
pub struct ListQuery {
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub page: QueryPage,
    #[serde(default)]
    pub page_size: QueryPageSize,
}

pub struct PreparedFilters {
    pub fields: Vec<FilterField>,
    pub filters: Vec<FilterClauseView>,
    pub errors: Vec<FilterRowError>,
    pub condition: Condition,
    /// Clauses the handler applies after computing line counts and totals.
    pub post: Vec<FilterClause>,
}

pub fn prepare(query: &str, fields: Vec<FilterField>, backend: DatabaseBackend) -> PreparedFilters {
    let clauses = parse_list_filters(query);
    let applied = apply_filters(backend, &fields, &clauses);
    let filters = FilterClauseView::from_clauses(&clauses, &applied.errors);
    PreparedFilters {
        fields,
        filters,
        errors: applied.errors,
        condition: applied.condition,
        post: applied.post,
    }
}

pub fn quotation_fields() -> Vec<FilterField> {
    vec![
        number_column("id", "Id", quotation::Column::Id),
        text_column(
            "invoice_number",
            "Quotation #",
            quotation::Column::InvoiceNumber,
        ),
        date_column("date", "Date", quotation::Column::Date),
        customer_field(quotation::Column::CustomerId),
        post_decimal("total", "Total"),
    ]
}

pub fn draft_work_order_fields() -> Vec<FilterField> {
    vec![
        number_column("id", "Id", draft_work_order::Column::Id),
        text_column(
            "order_number",
            "Order #",
            draft_work_order::Column::OrderNumber,
        ),
        customer_field(draft_work_order::Column::CustomerId),
        post_number("lines", "Lines"),
        post_decimal("total", "Total cost"),
    ]
}

pub fn issued_work_order_fields() -> Vec<FilterField> {
    vec![
        number_column("id", "Id", work_order::Column::Id),
        text_column("order_number", "Order #", work_order::Column::OrderNumber),
        customer_field(work_order::Column::CustomerId),
        number_column("job_id", "Job", work_order::Column::JobId),
        post_number("lines", "Lines"),
        post_decimal("total", "Total cost"),
    ]
}

pub fn component_fields() -> Vec<FilterField> {
    vec![
        number_column("id", "Id", component::Column::Id),
        text_column("name", "Name", component::Column::Name),
        post_text("variables", "Variables"),
        text_column(
            "cost_formula",
            "Cost formula",
            component::Column::CostFormula,
        ),
        text_column(
            "weight_formula",
            "Weight formula",
            component::Column::WeightFormula,
        ),
    ]
}

fn customer_field<C>(col: C) -> FilterField
where
    C: Copy + sea_orm::sea_query::IntoColumnRef + Send + Sync + 'static,
{
    foreign_key_column(
        "customer_id",
        "Customer",
        col,
        CompanyFkSelectRouteTag.url(),
        "Any customer…",
    )
}

fn post_number(key: &'static str, label: &'static str) -> FilterField {
    custom(
        key,
        label,
        FilterWidget::Number,
        Vec::new(),
        true,
        |clause, _| {
            let raw = clause.value();
            raw.parse::<i64>()
                .map_err(|_| format!("invalid number: {raw}"))?;
            Ok(Condition::all())
        },
    )
}

fn post_decimal(key: &'static str, label: &'static str) -> FilterField {
    custom(
        key,
        label,
        FilterWidget::Decimal,
        Vec::new(),
        true,
        |clause, _| {
            let raw = clause.value();
            raw.parse::<Decimal>()
                .map_err(|_| format!("invalid number: {raw}"))?;
            Ok(Condition::all())
        },
    )
}

fn post_text(key: &'static str, label: &'static str) -> FilterField {
    custom(
        key,
        label,
        FilterWidget::Text,
        Vec::new(),
        true,
        |clause, _| {
            if clause.value().trim().is_empty() {
                return Err("value is required".into());
            }
            Ok(Condition::all())
        },
    )
}

pub fn variables_label(value: &serde_json::Value) -> String {
    let entries = schema_entries_from_json(value);
    if entries.is_empty() {
        "—".into()
    } else {
        entries.join(", ")
    }
}

pub fn allows_i64(post: &[FilterClause], key: &str, value: i64) -> bool {
    post.iter()
        .filter(|clause| clause.field == key)
        .all(|clause| match clause.value().parse::<i64>() {
            Ok(needle) => cmp_ord(clause.op, value.cmp(&needle)),
            Err(_) => true,
        })
}

pub fn allows_decimal(post: &[FilterClause], key: &str, value: Decimal) -> bool {
    post.iter()
        .filter(|clause| clause.field == key)
        .all(|clause| match clause.value().parse::<Decimal>() {
            Ok(needle) => cmp_ord(clause.op, value.cmp(&needle)),
            Err(_) => true,
        })
}

pub fn allows_text(post: &[FilterClause], key: &str, haystacks: &[&str]) -> bool {
    post.iter()
        .filter(|clause| clause.field == key)
        .all(|clause| {
            let needle = clause.value().to_lowercase();
            haystacks
                .iter()
                .any(|hay| hay.to_lowercase().contains(&needle))
        })
}

fn cmp_ord(op: CmpOp, ordering: std::cmp::Ordering) -> bool {
    match op {
        CmpOp::Lt => ordering.is_lt(),
        CmpOp::Gt => ordering.is_gt(),
        CmpOp::Eq => ordering.is_eq(),
    }
}

pub fn paginate<T>(items: Vec<T>, page: u32, page_size: u32) -> (Vec<T>, u32, u32) {
    let page = page.max(1);
    let page_size = page_size.max(1);
    let total = items.len() as u32;
    let num_pages = (total.saturating_add(page_size - 1) / page_size).max(1);
    let start = ((page - 1) as usize).saturating_mul(page_size as usize);
    let page_items = items
        .into_iter()
        .skip(start)
        .take(page_size as usize)
        .collect();
    (page_items, page, num_pages)
}

pub async fn fill_company_fk(db: &DatabaseConnection, views: &mut [FilterClauseView], key: &str) {
    for view in views.iter_mut() {
        if view.clause.field != key {
            continue;
        }
        let Ok(id) = view.clause.value().parse::<i64>() else {
            continue;
        };
        if id <= 0 {
            continue;
        }
        view.fk_display = company::Entity::find_by_id(id)
            .one(db)
            .await
            .ok()
            .flatten()
            .map(|company| company.name)
            .unwrap_or_else(|| format!("Customer #{id}"));
    }
}
