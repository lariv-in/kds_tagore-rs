pub mod completed_jobs;
pub mod jobs;
pub mod machines;

pub use lariv_rs::web::ModalFormQuery as ModalNameQuery;

pub fn path_and_query(uri: &axum::http::Uri) -> String {
    uri.path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string())
}

#[derive(Debug, serde::Deserialize, Default)]
pub struct BulkIdsQuery {
    #[serde(default)]
    pub ids: Option<String>,
}

#[derive(Debug, serde::Deserialize, Default)]
pub struct BulkIdsForm {
    #[serde(default)]
    pub ids: String,
}

pub fn parse_bulk_ids(raw: &str) -> Vec<i64> {
    let mut ids: Vec<i64> = raw
        .split(',')
        .filter_map(|p| p.trim().parse().ok())
        .filter(|id| *id > 0)
        .collect();
    ids.sort_unstable();
    ids.dedup();
    ids
}

pub fn bulk_delete_message(noun: &str, count: usize) -> String {
    if count == 1 {
        format!("Are you sure you want to delete the selected {noun}?")
    } else {
        format!("Are you sure you want to delete {count} selected {noun}?")
    }
}
