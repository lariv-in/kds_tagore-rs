use lariv_rs::plugins::users::state::AuthContext;
use lariv_rs::web::opt_or_log;
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, Order, QueryFilter, QueryOrder, Select,
    sea_query::{Expr, SimpleExpr},
};

use super::entities::{
    completed_job::{self, Entity as CompletedJobEntity},
    job::{self, Entity as JobEntity},
    machine::{self, Entity as MachineEntity},
};

pub fn sql_job_not_completed() -> sea_orm::sea_query::SimpleExpr {
    Expr::cust(
        "NOT EXISTS (SELECT 1 FROM machinery_completed_jobs c WHERE c.job_id = machinery_jobs.id)",
    )
}

pub fn scope_superuser<T>(query: Select<T>, auth: &AuthContext) -> Select<T>
where
    T: EntityTrait,
{
    if auth.user.is_superuser {
        return query;
    }
    query.filter(Expr::cust("1 = 0"))
}

pub async fn find_machine_scoped(
    db: &DatabaseConnection,
    id: i64,
    auth: &AuthContext,
) -> Option<machine::Model> {
    opt_or_log(
        scope_superuser(MachineEntity::find_by_id(id), auth)
            .one(db)
            .await,
        "find machine",
    )
}

pub async fn find_job_scoped(
    db: &DatabaseConnection,
    id: i64,
    auth: &AuthContext,
) -> Option<job::Model> {
    opt_or_log(
        scope_superuser(JobEntity::find_by_id(id), auth)
            .one(db)
            .await,
        "find job",
    )
}

pub async fn find_open_job(
    db: &DatabaseConnection,
    id: i64,
    auth: &AuthContext,
) -> Option<job::Model> {
    opt_or_log(
        scope_superuser(JobEntity::find_by_id(id), auth)
            .filter(sql_job_not_completed())
            .one(db)
            .await,
        "find open job",
    )
}

pub async fn find_completed_job_scoped(
    db: &DatabaseConnection,
    id: i64,
    auth: &AuthContext,
) -> Option<completed_job::Model> {
    opt_or_log(
        scope_superuser(CompletedJobEntity::find_by_id(id), auth)
            .one(db)
            .await,
        "find completed job",
    )
}

pub fn apply_name_filter<E, C>(mut query: Select<E>, col: C, name: Option<&str>) -> Select<E>
where
    E: EntityTrait,
    C: ColumnTrait,
{
    if let Some(n) = name.filter(|s| !s.is_empty()) {
        query = query.filter(col.contains(n));
    }
    query
}

pub fn apply_name_sort_or_id_desc<E>(
    query: Select<E>,
    name_col: E::Column,
    id_col: E::Column,
    sort: Option<&str>,
) -> Select<E>
where
    E: EntityTrait,
    E::Column: ColumnTrait,
{
    match sort.unwrap_or("").trim() {
        s if s.eq_ignore_ascii_case("Name DESC") => query.order_by_desc(name_col),
        s if s.eq_ignore_ascii_case("Name ASC") || s.eq_ignore_ascii_case("Name") => {
            query.order_by_asc(name_col)
        }
        _ => query.order_by_desc(id_col),
    }
}

fn hub_sort_desc(sort: Option<&str>, key: &str) -> Option<bool> {
    let s = sort.unwrap_or("").trim();
    if s.is_empty() {
        return None;
    }
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.first().is_none_or(|col| !col.eq_ignore_ascii_case(key)) {
        return None;
    }
    Some(parts.get(1).is_some_and(|d| d.eq_ignore_ascii_case("DESC")))
}

fn order_col<E, C>(query: Select<E>, col: C, desc: bool) -> Select<E>
where
    E: EntityTrait,
    C: ColumnTrait,
{
    if desc {
        query.order_by_desc(col)
    } else {
        query.order_by_asc(col)
    }
}

fn order_expr<E>(query: Select<E>, expr: SimpleExpr, desc: bool) -> Select<E>
where
    E: EntityTrait,
{
    query.order_by(expr, if desc { Order::Desc } else { Order::Asc })
}

fn expr_job_machine_count(job_id_sql: &str) -> SimpleExpr {
    Expr::cust(format!(
        "(SELECT COUNT(*) FROM machinery_job_machines m WHERE m.job_id = {job_id_sql})"
    ))
}

pub fn apply_open_job_hub_sort(
    query: Select<job::Entity>,
    sort: Option<&str>,
) -> Select<job::Entity> {
    let query = if let Some(desc) = hub_sort_desc(sort, "Name") {
        order_col(query, job::Column::Name, desc)
    } else if let Some(desc) = hub_sort_desc(sort, "Duration") {
        order_col(query, job::Column::Duration, desc)
    } else if let Some(desc) = hub_sort_desc(sort, "Progress") {
        order_col(query, job::Column::Progress, desc)
    } else if let Some(desc) = hub_sort_desc(sort, "Order") {
        order_col(query, job::Column::Order, desc)
    } else if let Some(desc) = hub_sort_desc(sort, "Machines") {
        order_expr(query, expr_job_machine_count("machinery_jobs.id"), desc)
    } else {
        return query.order_by_desc(job::Column::Id);
    };
    query.order_by_desc(job::Column::Id)
}

pub fn apply_completed_job_hub_sort(
    query: Select<completed_job::Entity>,
    sort: Option<&str>,
) -> Select<completed_job::Entity> {
    let query = if let Some(desc) = hub_sort_desc(sort, "Name") {
        order_col(query, job::Column::Name, desc)
    } else if let Some(desc) = hub_sort_desc(sort, "Duration") {
        order_col(query, job::Column::Duration, desc)
    } else if let Some(desc) = hub_sort_desc(sort, "Progress") {
        order_col(query, job::Column::Progress, desc)
    } else if let Some(desc) = hub_sort_desc(sort, "Order") {
        order_col(query, job::Column::Order, desc)
    } else if let Some(desc) = hub_sort_desc(sort, "Machines") {
        order_expr(
            query,
            expr_job_machine_count("machinery_completed_jobs.job_id"),
            desc,
        )
    } else if let Some(desc) = hub_sort_desc(sort, "Completed") {
        order_col(query, completed_job::Column::CompletedAt, desc)
    } else {
        return query.order_by_desc(completed_job::Column::Id);
    };
    query.order_by_desc(completed_job::Column::Id)
}
