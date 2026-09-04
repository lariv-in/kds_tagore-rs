use std::collections::BTreeSet;

use chrono::{DateTime, Duration, Utc};
use lariv_rs::duration::{format_duration, parse_duration};
use lariv_rs::plugins::users::state::AuthContext;
use lariv_rs::web::opt_or_log;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DatabaseConnection,
    EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, TransactionTrait,
};

use super::duration::JobDuration;
use super::entities::{
    completed_job::{self, Entity as CompletedJobEntity},
    job::{self, Entity as JobEntity},
    job_file, job_machine,
    machine::{self, Entity as MachineEntity},
};
use super::scope::{
    find_completed_job_scoped, find_job_scoped, find_open_job, sql_job_not_completed,
};

pub fn clamp_progress(value: i64) -> i16 {
    value.clamp(0, 100) as i16
}

pub fn parse_job_progress(value: i64) -> Result<i16, String> {
    if !(0..=100).contains(&value) {
        return Err("Progress must be a percentage between 0 and 100".into());
    }
    Ok(value as i16)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrderMove {
    Up,
    Down,
}

pub fn next_order_after_move(current: i64, orders: &[i64], dir: OrderMove) -> Option<i64> {
    match dir {
        OrderMove::Up => orders
            .iter()
            .copied()
            .filter(|order| *order > current)
            .min()
            .map(|neighbor| neighbor + 1),
        OrderMove::Down => orders
            .iter()
            .copied()
            .filter(|order| *order < current)
            .max()
            .map(|neighbor| neighbor - 1),
    }
}

pub fn parse_job_duration(raw: &str) -> Result<JobDuration, String> {
    parse_duration(raw).map(JobDuration::from_nanos)
}

pub fn format_job_duration(duration: JobDuration) -> String {
    format_duration(duration.num_nanoseconds())
}

pub fn remaining_duration(duration: Duration, progress: i16) -> Duration {
    let progress = i32::from(progress.clamp(0, 100));
    duration * (100 - progress) / 100
}

pub fn machine_free_on(now: DateTime<Utc>, remaining: Duration) -> DateTime<Utc> {
    now.checked_add_signed(remaining).unwrap_or(now)
}

pub async fn err_if_job_completed(db: &DatabaseConnection, job_id: i64) -> Result<(), String> {
    let completed = CompletedJobEntity::find()
        .filter(completed_job::Column::JobId.eq(job_id))
        .count(db)
        .await
        .map_err(|e| e.to_string())?;
    if completed > 0 {
        return Err("job is completed and cannot be changed".to_string());
    }
    Ok(())
}

async fn validate_machine_ids<C: ConnectionTrait>(db: &C, ids: &[i64]) -> Result<(), String> {
    let unique: BTreeSet<i64> = ids.iter().copied().filter(|id| *id > 0).collect();
    if unique.is_empty() {
        return Ok(());
    }
    let found = MachineEntity::find()
        .filter(machine::Column::Id.is_in(unique.iter().copied()))
        .all(db)
        .await
        .map_err(|e| e.to_string())?;
    if found.len() != unique.len() {
        return Err("Select a valid machine".into());
    }
    Ok(())
}

pub async fn sync_job_machines<C: ConnectionTrait>(
    db: &C,
    job_id: i64,
    machine_ids: &[i64],
) -> Result<(), String> {
    validate_machine_ids(db, machine_ids).await?;
    job_machine::Entity::delete_many()
        .filter(job_machine::Column::JobId.eq(job_id))
        .exec(db)
        .await
        .map_err(|e| e.to_string())?;
    let mut seen = BTreeSet::new();
    for machine_id in machine_ids.iter().copied().filter(|id| *id > 0) {
        if !seen.insert(machine_id) {
            continue;
        }
        job_machine::ActiveModel {
            job_id: Set(job_id),
            machine_id: Set(machine_id),
        }
        .insert(db)
        .await
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub async fn sync_job_files<C: ConnectionTrait>(
    db: &C,
    job_id: i64,
    file_ids: &[i64],
) -> Result<(), String> {
    job_file::Entity::delete_many()
        .filter(job_file::Column::JobId.eq(job_id))
        .exec(db)
        .await
        .map_err(|e| e.to_string())?;
    let mut seen = BTreeSet::new();
    for v_node_id in file_ids.iter().copied().filter(|id| *id > 0) {
        if !seen.insert(v_node_id) {
            continue;
        }
        job_file::ActiveModel {
            job_id: Set(job_id),
            v_node_id: Set(v_node_id),
        }
        .insert(db)
        .await
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub async fn load_job_machine_ids<C: ConnectionTrait>(db: &C, job_id: i64) -> Vec<i64> {
    job_machine::Entity::find()
        .filter(job_machine::Column::JobId.eq(job_id))
        .order_by_asc(job_machine::Column::MachineId)
        .all(db)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|row| row.machine_id)
        .collect()
}

pub async fn load_job_file_ids<C: ConnectionTrait>(db: &C, job_id: i64) -> Vec<i64> {
    job_file::Entity::find()
        .filter(job_file::Column::JobId.eq(job_id))
        .order_by_asc(job_file::Column::VNodeId)
        .all(db)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|row| row.v_node_id)
        .collect()
}

pub async fn complete_job_if_needed<C: ConnectionTrait>(
    db: &C,
    job_id: i64,
    progress: i16,
) -> Result<Option<completed_job::Model>, String> {
    if progress < 100 {
        return Ok(None);
    }
    if let Some(existing) = CompletedJobEntity::find()
        .filter(completed_job::Column::JobId.eq(job_id))
        .one(db)
        .await
        .map_err(|e| e.to_string())?
    {
        return Ok(Some(existing));
    }
    let now = Utc::now();
    let completed = completed_job::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        job_id: Set(job_id),
        completed_at: Set(now),
    }
    .insert(db)
    .await
    .map_err(|e| e.to_string())?;
    Ok(Some(completed))
}

pub async fn delete_open_job(
    db: &DatabaseConnection,
    job_id: i64,
    auth: &AuthContext,
) -> Result<(), String> {
    let job = find_open_job(db, job_id, auth)
        .await
        .ok_or_else(|| "job not found or already completed".to_string())?;
    err_if_job_completed(db, job.id).await?;
    JobEntity::delete_by_id(job.id)
        .exec(db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn delete_completed_job(
    db: &DatabaseConnection,
    completed_job_id: i64,
    auth: &AuthContext,
) -> Result<(), String> {
    let completed = find_completed_job_scoped(db, completed_job_id, auth)
        .await
        .ok_or_else(|| "completed job not found".to_string())?;
    JobEntity::delete_by_id(completed.job_id)
        .exec(db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn job_is_open(db: &DatabaseConnection, job_id: i64) -> bool {
    JobEntity::find_by_id(job_id)
        .filter(sql_job_not_completed())
        .one(db)
        .await
        .ok()
        .flatten()
        .is_some()
}

pub async fn completed_job_id_for_job(db: &DatabaseConnection, job_id: i64) -> Option<i64> {
    opt_or_log(
        CompletedJobEntity::find()
            .filter(completed_job::Column::JobId.eq(job_id))
            .one(db)
            .await,
        "find completed job for job",
    )
    .map(|c| c.id)
}

pub async fn open_job_orders(db: &DatabaseConnection) -> Vec<i64> {
    JobEntity::find()
        .filter(sql_job_not_completed())
        .all(db)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|job| job.order)
        .collect()
}

pub async fn move_open_job_order(
    db: &DatabaseConnection,
    job_id: i64,
    auth: &AuthContext,
    dir: OrderMove,
) -> Result<job::Model, String> {
    let job = find_open_job(db, job_id, auth)
        .await
        .ok_or_else(|| "job not found or already completed".to_string())?;
    let orders = open_job_orders(db).await;
    let new_order = next_order_after_move(job.order, &orders, dir)
        .ok_or_else(|| match dir {
            OrderMove::Up => "job is already at the top of the order".to_string(),
            OrderMove::Down => "job is already at the bottom of the order".to_string(),
        })?;
    let mut am: job::ActiveModel = job.into();
    am.order = Set(new_order);
    am.updated_at = Set(Some(Utc::now()));
    am.update(db).await.map_err(|e| e.to_string())
}

pub async fn duplicate_job(
    db: &DatabaseConnection,
    job_id: i64,
    auth: &AuthContext,
) -> Result<job::Model, String> {
    let source = find_job_scoped(db, job_id, auth)
        .await
        .ok_or_else(|| "job not found".to_string())?;
    let machine_ids = load_job_machine_ids(db, source.id).await;
    let file_ids = load_job_file_ids(db, source.id).await;

    let txn = db.begin().await.map_err(|e| e.to_string())?;
    let now = Utc::now();
    let created = job::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        name: Set(source.name.clone()),
        duration: Set(source.duration),
        progress: Set(0),
        order: Set(source.order),
        remarks: Set(source.remarks.clone()),
    }
    .insert(&txn)
    .await
    .map_err(|e| e.to_string())?;
    sync_job_machines(&txn, created.id, &machine_ids).await?;
    sync_job_files(&txn, created.id, &file_ids).await?;
    txn.commit().await.map_err(|e| e.to_string())?;
    Ok(created)
}

pub async fn jobs_for_machine<C: ConnectionTrait>(
    db: &C,
    machine_id: i64,
) -> Result<Vec<job::Model>, String> {
    let links = job_machine::Entity::find()
        .filter(job_machine::Column::MachineId.eq(machine_id))
        .all(db)
        .await
        .map_err(|e| e.to_string())?;
    if links.is_empty() {
        return Ok(Vec::new());
    }
    let job_ids: Vec<i64> = links.into_iter().map(|l| l.job_id).collect();
    let mut jobs = JobEntity::find()
        .filter(job::Column::Id.is_in(job_ids))
        .order_by_asc(job::Column::Order)
        .order_by_asc(job::Column::Id)
        .all(db)
        .await
        .map_err(|e| e.to_string())?;
    jobs.sort_by_key(|j| (j.order, j.id));
    Ok(jobs)
}

pub async fn machine_remaining_duration<C: ConnectionTrait>(
    db: &C,
    machine_id: i64,
) -> Result<Duration, String> {
    let jobs = jobs_for_machine(db, machine_id).await?;
    let mut total = Duration::zero();
    for job in jobs {
        if CompletedJobEntity::find()
            .filter(completed_job::Column::JobId.eq(job.id))
            .count(db)
            .await
            .map_err(|e| e.to_string())?
            > 0
        {
            continue;
        }
        total = total + remaining_duration(job.duration.inner(), job.progress);
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remaining_duration_scales_by_progress() {
        let hour = Duration::hours(1);
        assert_eq!(remaining_duration(hour, 0), hour);
        assert_eq!(remaining_duration(hour, 50), Duration::minutes(30));
        assert_eq!(remaining_duration(hour, 100), Duration::zero());
        assert_eq!(remaining_duration(hour, 200), Duration::zero());
    }

    #[test]
    fn clamp_progress_stays_in_range() {
        assert_eq!(clamp_progress(-4), 0);
        assert_eq!(clamp_progress(40), 40);
        assert_eq!(clamp_progress(100), 100);
        assert_eq!(clamp_progress(140), 100);
    }

    #[test]
    fn parse_job_progress_rejects_out_of_range() {
        assert_eq!(parse_job_progress(0).unwrap(), 0);
        assert_eq!(parse_job_progress(100).unwrap(), 100);
        assert!(parse_job_progress(-1).is_err());
        assert!(parse_job_progress(101).is_err());
    }

    #[test]
    fn next_order_after_move_steps_to_neighbor() {
        let orders = [1, 10, 20];
        assert_eq!(next_order_after_move(1, &orders, OrderMove::Up), Some(11));
        assert_eq!(next_order_after_move(10, &orders, OrderMove::Up), Some(21));
        assert_eq!(next_order_after_move(20, &orders, OrderMove::Up), None);
        assert_eq!(next_order_after_move(20, &orders, OrderMove::Down), Some(9));
        assert_eq!(next_order_after_move(10, &orders, OrderMove::Down), Some(0));
        assert_eq!(next_order_after_move(1, &orders, OrderMove::Down), None);
    }

    #[test]
    fn parse_job_duration_rejects_empty() {
        assert!(parse_job_duration("").is_err());
        assert!(parse_job_duration("30m").unwrap().num_nanoseconds() > 0);
    }

    #[test]
    fn machine_free_on_adds_remaining() {
        let now = DateTime::parse_from_rfc3339("2026-09-05T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        assert_eq!(machine_free_on(now, Duration::zero()), now);
        assert_eq!(
            machine_free_on(now, Duration::hours(2)),
            now + Duration::hours(2)
        );
    }
}
