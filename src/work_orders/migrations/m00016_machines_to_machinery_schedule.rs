use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

async fn exec(db: &impl ConnectionTrait, backend: DbBackend, sql: &str) -> Result<(), DbErr> {
    db.execute_raw(Statement::from_string(backend, sql.to_string()))
        .await?;
    Ok(())
}

async fn table_exists(
    db: &impl ConnectionTrait,
    backend: DbBackend,
    table: &str,
) -> Result<bool, DbErr> {
    if backend == DbBackend::Postgres {
        let sql = format!(
            "SELECT COUNT(*) AS n FROM information_schema.tables WHERE table_schema = 'public' AND table_name = '{table}'"
        );
        let row = db
            .query_one_raw(Statement::from_string(backend, sql))
            .await?;
        let n: i64 = row.map(|r| r.try_get("", "n").unwrap_or(0)).unwrap_or(0);
        Ok(n > 0)
    } else {
        let sql =
            format!("SELECT 1 AS n FROM sqlite_master WHERE type = 'table' AND name = '{table}'");
        let row = db
            .query_one_raw(Statement::from_string(backend, sql))
            .await?;
        Ok(row.is_some())
    }
}

async fn copy_work_order_machines(
    db: &impl ConnectionTrait,
    backend: DbBackend,
) -> Result<Vec<(i64, i64)>, DbErr> {
    let rows = db
        .query_all_raw(Statement::from_string(
            backend,
            "SELECT id FROM work_order_machines ORDER BY id".to_string(),
        ))
        .await?;

    let mut map = Vec::with_capacity(rows.len());
    for row in rows {
        let old_id: i64 = row.try_get("", "id")?;
        let insert_sql = format!(
            "INSERT INTO machinery_machines (created_at, updated_at, name, rate_decimal) \
             SELECT created_at, updated_at, name, rate_decimal FROM work_order_machines WHERE id = {old_id} \
             RETURNING id"
        );
        let inserted = db
            .query_one_raw(Statement::from_string(backend, insert_sql))
            .await?
            .ok_or_else(|| DbErr::Custom(format!("failed to copy work_order_machine {old_id}")))?;
        let new_id: i64 = inserted.try_get("", "id")?;
        map.push((old_id, new_id));
    }
    Ok(map)
}

async fn remap_line_machine_ids(
    db: &impl ConnectionTrait,
    backend: DbBackend,
    map: &[(i64, i64)],
) -> Result<(), DbErr> {
    for (old_id, new_id) in map {
        if old_id == new_id {
            continue;
        }
        exec(
            db,
            backend,
            &format!(
                "UPDATE draft_work_order_machine_lines SET machine_id = {new_id} WHERE machine_id = {old_id}"
            ),
        )
        .await?;
        exec(
            db,
            backend,
            &format!(
                "UPDATE kds_quotation_machine_lines SET machine_id = {new_id} WHERE machine_id = {old_id}"
            ),
        )
        .await?;
    }
    Ok(())
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        let backend = db.get_database_backend();

        if !table_exists(db, backend, "work_order_machines").await? {
            return Ok(());
        }

        if !table_exists(db, backend, "machinery_machines").await? {
            if backend == DbBackend::Postgres {
                exec(
                    db,
                    backend,
                    r#"
                    CREATE TABLE machinery_machines (
                        id bigserial PRIMARY KEY,
                        created_at timestamp with time zone,
                        updated_at timestamp with time zone,
                        name text NOT NULL,
                        rate_decimal numeric(16,4) NOT NULL DEFAULT 0
                    )
                    "#,
                )
                .await?;
            } else {
                exec(
                    db,
                    backend,
                    r#"
                    CREATE TABLE machinery_machines (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        created_at TEXT,
                        updated_at TEXT,
                        name TEXT NOT NULL,
                        rate_decimal REAL NOT NULL DEFAULT 0
                    )
                    "#,
                )
                .await?;
            }
        }

        if backend == DbBackend::Postgres {
            exec(
                db,
                backend,
                "ALTER TABLE draft_work_order_machine_lines DROP CONSTRAINT IF EXISTS draft_work_order_machine_lines_machine_id_fkey",
            )
            .await?;

            let map = copy_work_order_machines(db, backend).await?;
            remap_line_machine_ids(db, backend, &map).await?;

            exec(
                db,
                backend,
                r#"
                ALTER TABLE draft_work_order_machine_lines
                    ADD CONSTRAINT draft_work_order_machine_lines_machine_id_fkey
                    FOREIGN KEY (machine_id) REFERENCES machinery_machines(id) ON DELETE RESTRICT
                "#,
            )
            .await?;
            exec(db, backend, "DROP TABLE IF EXISTS work_order_machines").await?;
        } else {
            let _ = exec(db, backend, "PRAGMA foreign_keys = OFF").await;
            let map = copy_work_order_machines(db, backend).await?;
            remap_line_machine_ids(db, backend, &map).await?;

            exec(
                db,
                backend,
                r#"
                CREATE TABLE draft_work_order_machine_lines_new (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    created_at TEXT,
                    updated_at TEXT,
                    draft_work_order_id INTEGER NOT NULL REFERENCES draft_work_orders(id) ON DELETE CASCADE,
                    machine_id INTEGER NOT NULL REFERENCES machinery_machines(id) ON DELETE RESTRICT,
                    rate_decimal NUMERIC NOT NULL DEFAULT 0,
                    time_used INTEGER NOT NULL DEFAULT 0
                )
                "#,
            )
            .await?;
            exec(
                db,
                backend,
                "INSERT INTO draft_work_order_machine_lines_new SELECT * FROM draft_work_order_machine_lines",
            )
            .await?;
            exec(db, backend, "DROP TABLE draft_work_order_machine_lines").await?;
            exec(
                db,
                backend,
                "ALTER TABLE draft_work_order_machine_lines_new RENAME TO draft_work_order_machine_lines",
            )
            .await?;
            exec(
                db,
                backend,
                "CREATE INDEX IF NOT EXISTS idx_draft_work_order_machine_lines_wo ON draft_work_order_machine_lines (draft_work_order_id)",
            )
            .await?;
            exec(db, backend, "DROP TABLE IF EXISTS work_order_machines").await?;
            let _ = exec(db, backend, "PRAGMA foreign_keys = ON").await;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        let backend = db.get_database_backend();

        if table_exists(db, backend, "work_order_machines").await? {
            return Ok(());
        }

        if backend == DbBackend::Postgres {
            exec(
                db,
                backend,
                r#"
                CREATE TABLE IF NOT EXISTS work_order_machines (
                    id bigserial PRIMARY KEY,
                    created_at timestamp with time zone,
                    updated_at timestamp with time zone,
                    name text NOT NULL,
                    rate_decimal numeric(16,4) NOT NULL
                )
                "#,
            )
            .await?;
            exec(
                db,
                backend,
                r#"
                INSERT INTO work_order_machines (id, created_at, updated_at, name, rate_decimal)
                SELECT id, created_at, updated_at, name, rate_decimal FROM machinery_machines
                "#,
            )
            .await?;
            exec(
                db,
                backend,
                r#"
                SELECT setval(
                    pg_get_serial_sequence('work_order_machines', 'id'),
                    COALESCE((SELECT MAX(id) FROM work_order_machines), 1)
                )
                "#,
            )
            .await?;
            exec(
                db,
                backend,
                "ALTER TABLE draft_work_order_machine_lines DROP CONSTRAINT IF EXISTS draft_work_order_machine_lines_machine_id_fkey",
            )
            .await?;
            exec(
                db,
                backend,
                r#"
                ALTER TABLE draft_work_order_machine_lines
                    ADD CONSTRAINT draft_work_order_machine_lines_machine_id_fkey
                    FOREIGN KEY (machine_id) REFERENCES work_order_machines(id) ON DELETE RESTRICT
                "#,
            )
            .await?;
        } else {
            let _ = exec(db, backend, "PRAGMA foreign_keys = OFF").await;
            exec(
                db,
                backend,
                r#"
                CREATE TABLE work_order_machines (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    created_at TEXT,
                    updated_at TEXT,
                    name TEXT NOT NULL,
                    rate_decimal NUMERIC NOT NULL
                )
                "#,
            )
            .await?;
            exec(
                db,
                backend,
                r#"
                INSERT INTO work_order_machines (id, created_at, updated_at, name, rate_decimal)
                SELECT id, created_at, updated_at, name, rate_decimal FROM machinery_machines
                "#,
            )
            .await?;
            exec(
                db,
                backend,
                r#"
                CREATE TABLE draft_work_order_machine_lines_new (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    created_at TEXT,
                    updated_at TEXT,
                    draft_work_order_id INTEGER NOT NULL REFERENCES draft_work_orders(id) ON DELETE CASCADE,
                    machine_id INTEGER NOT NULL REFERENCES work_order_machines(id) ON DELETE RESTRICT,
                    rate_decimal NUMERIC NOT NULL DEFAULT 0,
                    time_used INTEGER NOT NULL DEFAULT 0
                )
                "#,
            )
            .await?;
            exec(
                db,
                backend,
                "INSERT INTO draft_work_order_machine_lines_new SELECT * FROM draft_work_order_machine_lines",
            )
            .await?;
            exec(db, backend, "DROP TABLE draft_work_order_machine_lines").await?;
            exec(
                db,
                backend,
                "ALTER TABLE draft_work_order_machine_lines_new RENAME TO draft_work_order_machine_lines",
            )
            .await?;
            exec(
                db,
                backend,
                "CREATE INDEX IF NOT EXISTS idx_draft_work_order_machine_lines_wo ON draft_work_order_machine_lines (draft_work_order_id)",
            )
            .await?;
            let _ = exec(db, backend, "PRAGMA foreign_keys = ON").await;
        }

        Ok(())
    }
}
