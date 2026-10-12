//! Catalogs for machinery-schedule table and picker filters.

use sea_orm::DatabaseBackend;
use sea_orm::sea_query::Condition;

use lariv_core::components::FilterClauseView;
use lariv_core::list_filters::{
    FilterField, FilterRowError, apply_filters, number_column, parse_list_filters, text_column,
};

use crate::entities::{job, machine};

pub struct PreparedFilters {
    pub fields: Vec<FilterField>,
    pub filters: Vec<FilterClauseView>,
    pub errors: Vec<FilterRowError>,
    pub condition: Condition,
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
    }
}

pub fn machine_fields() -> Vec<FilterField> {
    vec![
        number_column("id", "Id", machine::Column::Id),
        text_column("name", "Name", machine::Column::Name),
        text_column("cost_formula", "Formula", machine::Column::CostFormula),
    ]
}

pub fn job_fields() -> Vec<FilterField> {
    vec![text_column("name", "Name", job::Column::Name)]
}
