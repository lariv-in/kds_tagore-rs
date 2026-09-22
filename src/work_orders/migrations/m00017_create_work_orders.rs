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

async fn ensure_machinery_job_tables(
    db: &impl ConnectionTrait,
    backend: DbBackend,
) -> Result<(), DbErr> {
    if table_exists(db, backend, "machinery_jobs").await? {
        return Ok(());
    }
    if backend == DbBackend::Postgres {
        exec(
            db,
            backend,
            r#"
            CREATE TABLE machinery_jobs (
                id bigserial PRIMARY KEY,
                created_at timestamp with time zone,
                updated_at timestamp with time zone,
                name text NOT NULL,
                duration bigint NOT NULL,
                progress smallint NOT NULL,
                "order" bigint NOT NULL,
                remarks text NOT NULL,
                source_doc_type text NOT NULL DEFAULT '',
                source_doc_id bigint NOT NULL DEFAULT 0
            )
            "#,
        )
        .await?;
        exec(
            db,
            backend,
            r#"
            CREATE TABLE IF NOT EXISTS machinery_job_machines (
                job_id bigint NOT NULL REFERENCES machinery_jobs(id) ON DELETE CASCADE,
                machine_id bigint NOT NULL REFERENCES machinery_machines(id) ON DELETE RESTRICT,
                PRIMARY KEY (job_id, machine_id)
            )
            "#,
        )
        .await?;
        exec(
            db,
            backend,
            r#"
            CREATE TABLE IF NOT EXISTS machinery_completed_jobs (
                id bigserial PRIMARY KEY,
                created_at timestamp with time zone,
                updated_at timestamp with time zone,
                job_id bigint NOT NULL REFERENCES machinery_jobs(id) ON DELETE CASCADE,
                completed_at timestamp with time zone NOT NULL
            )
            "#,
        )
        .await?;
        exec(
            db,
            backend,
            "CREATE UNIQUE INDEX IF NOT EXISTS uix_machinery_completed_jobs_job_id ON machinery_completed_jobs (job_id)",
        )
        .await?;
    } else {
        exec(
            db,
            backend,
            r#"
            CREATE TABLE machinery_jobs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                created_at TEXT,
                updated_at TEXT,
                name TEXT NOT NULL,
                duration INTEGER NOT NULL,
                progress INTEGER NOT NULL,
                "order" INTEGER NOT NULL,
                remarks TEXT NOT NULL,
                source_doc_type TEXT NOT NULL DEFAULT '',
                source_doc_id INTEGER NOT NULL DEFAULT 0
            )
            "#,
        )
        .await?;
        exec(
            db,
            backend,
            r#"
            CREATE TABLE IF NOT EXISTS machinery_job_machines (
                job_id INTEGER NOT NULL REFERENCES machinery_jobs(id) ON DELETE CASCADE,
                machine_id INTEGER NOT NULL REFERENCES machinery_machines(id) ON DELETE RESTRICT,
                PRIMARY KEY (job_id, machine_id)
            )
            "#,
        )
        .await?;
        exec(
            db,
            backend,
            r#"
            CREATE TABLE IF NOT EXISTS machinery_completed_jobs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                created_at TEXT,
                updated_at TEXT,
                job_id INTEGER NOT NULL REFERENCES machinery_jobs(id) ON DELETE CASCADE,
                completed_at TEXT NOT NULL
            )
            "#,
        )
        .await?;
        exec(
            db,
            backend,
            "CREATE UNIQUE INDEX IF NOT EXISTS uix_machinery_completed_jobs_job_id ON machinery_completed_jobs (job_id)",
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

        ensure_machinery_job_tables(db, backend).await?;

        if backend == DbBackend::Postgres {
            exec(
                db,
                backend,
                r#"
                CREATE TABLE IF NOT EXISTS kds_work_orders (
                    id bigserial PRIMARY KEY,
                    created_at timestamp with time zone,
                    updated_at timestamp with time zone,
                    order_number text NOT NULL,
                    customer_id bigint NOT NULL,
                    quotation_id bigint NULL REFERENCES kds_quotations(id) ON DELETE SET NULL,
                    job_id bigint NULL REFERENCES machinery_jobs(id) ON DELETE SET NULL
                )
                "#,
            )
            .await?;
            exec(
                db,
                backend,
                "CREATE INDEX IF NOT EXISTS idx_kds_work_orders_quotation ON kds_work_orders (quotation_id);",
            )
            .await?;
            exec(
                db,
                backend,
                "CREATE INDEX IF NOT EXISTS idx_kds_work_orders_job ON kds_work_orders (job_id);",
            )
            .await?;
            exec(
                db,
                backend,
                r#"
                CREATE TABLE IF NOT EXISTS kds_work_order_material_lines (
                    id bigserial PRIMARY KEY,
                    created_at timestamp with time zone,
                    updated_at timestamp with time zone,
                    work_order_id bigint NOT NULL REFERENCES kds_work_orders(id) ON DELETE CASCADE,
                    component_id bigint NOT NULL REFERENCES work_order_components(id) ON DELETE RESTRICT,
                    variables json NOT NULL DEFAULT '{}',
                    quantity numeric(16,4) NOT NULL DEFAULT 1,
                    unit_weight numeric(16,4) NOT NULL DEFAULT 0,
                    material_rate numeric(16,4) NOT NULL DEFAULT 0,
                    final_cost numeric(16,4) NOT NULL DEFAULT 0,
                    extra_data json NOT NULL DEFAULT '{}'
                )
                "#,
            )
            .await?;
            exec(
                db,
                backend,
                r#"
                CREATE TABLE IF NOT EXISTS kds_work_order_machine_lines (
                    id bigserial PRIMARY KEY,
                    created_at timestamp with time zone,
                    updated_at timestamp with time zone,
                    work_order_id bigint NOT NULL REFERENCES kds_work_orders(id) ON DELETE CASCADE,
                    machine_id bigint NULL REFERENCES machinery_machines(id) ON DELETE SET NULL,
                    name text NOT NULL,
                    time_used bigint NOT NULL,
                    rate_decimal numeric(16,4) NOT NULL
                )
                "#,
            )
            .await?;
            exec(
                db,
                backend,
                r#"
                CREATE TABLE IF NOT EXISTS kds_work_order_material_line_taxes (
                    line_id bigint NOT NULL REFERENCES kds_work_order_material_lines(id) ON DELETE CASCADE,
                    tax_id bigint NOT NULL REFERENCES taxes(id) ON DELETE RESTRICT,
                    PRIMARY KEY (line_id, tax_id)
                )
                "#,
            )
            .await?;
            exec(
                db,
                backend,
                r#"
                CREATE TABLE IF NOT EXISTS kds_work_order_machine_line_taxes (
                    line_id bigint NOT NULL REFERENCES kds_work_order_machine_lines(id) ON DELETE CASCADE,
                    tax_id bigint NOT NULL REFERENCES taxes(id) ON DELETE RESTRICT,
                    PRIMARY KEY (line_id, tax_id)
                )
                "#,
            )
            .await?;
        } else {
            exec(
                db,
                backend,
                r#"
                CREATE TABLE IF NOT EXISTS kds_work_orders (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    created_at TEXT,
                    updated_at TEXT,
                    order_number TEXT NOT NULL,
                    customer_id INTEGER NOT NULL,
                    quotation_id INTEGER NULL REFERENCES kds_quotations(id) ON DELETE SET NULL,
                    job_id INTEGER NULL REFERENCES machinery_jobs(id) ON DELETE SET NULL
                )
                "#,
            )
            .await?;
            exec(
                db,
                backend,
                "CREATE INDEX IF NOT EXISTS idx_kds_work_orders_quotation ON kds_work_orders (quotation_id);",
            )
            .await?;
            exec(
                db,
                backend,
                "CREATE INDEX IF NOT EXISTS idx_kds_work_orders_job ON kds_work_orders (job_id);",
            )
            .await?;
            exec(
                db,
                backend,
                r#"
                CREATE TABLE IF NOT EXISTS kds_work_order_material_lines (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    created_at TEXT,
                    updated_at TEXT,
                    work_order_id INTEGER NOT NULL REFERENCES kds_work_orders(id) ON DELETE CASCADE,
                    component_id INTEGER NOT NULL REFERENCES work_order_components(id) ON DELETE RESTRICT,
                    variables TEXT NOT NULL DEFAULT '{}',
                    quantity REAL NOT NULL DEFAULT 1,
                    unit_weight REAL NOT NULL DEFAULT 0,
                    material_rate REAL NOT NULL DEFAULT 0,
                    final_cost REAL NOT NULL DEFAULT 0,
                    extra_data TEXT NOT NULL DEFAULT '{}'
                )
                "#,
            )
            .await?;
            exec(
                db,
                backend,
                r#"
                CREATE TABLE IF NOT EXISTS kds_work_order_machine_lines (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    created_at TEXT,
                    updated_at TEXT,
                    work_order_id INTEGER NOT NULL REFERENCES kds_work_orders(id) ON DELETE CASCADE,
                    machine_id INTEGER NULL REFERENCES machinery_machines(id) ON DELETE SET NULL,
                    name TEXT NOT NULL,
                    time_used INTEGER NOT NULL,
                    rate_decimal REAL NOT NULL
                )
                "#,
            )
            .await?;
            exec(
                db,
                backend,
                r#"
                CREATE TABLE IF NOT EXISTS kds_work_order_material_line_taxes (
                    line_id INTEGER NOT NULL REFERENCES kds_work_order_material_lines(id) ON DELETE CASCADE,
                    tax_id INTEGER NOT NULL REFERENCES taxes(id) ON DELETE RESTRICT,
                    PRIMARY KEY (line_id, tax_id)
                )
                "#,
            )
            .await?;
            exec(
                db,
                backend,
                r#"
                CREATE TABLE IF NOT EXISTS kds_work_order_machine_line_taxes (
                    line_id INTEGER NOT NULL REFERENCES kds_work_order_machine_lines(id) ON DELETE CASCADE,
                    tax_id INTEGER NOT NULL REFERENCES taxes(id) ON DELETE RESTRICT,
                    PRIMARY KEY (line_id, tax_id)
                )
                "#,
            )
            .await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        let backend = db.get_database_backend();
        for table in [
            "kds_work_order_machine_line_taxes",
            "kds_work_order_material_line_taxes",
            "kds_work_order_machine_lines",
            "kds_work_order_material_lines",
            "kds_work_orders",
        ] {
            exec(db, backend, &format!("DROP TABLE IF EXISTS {table};")).await?;
        }
        Ok(())
    }
}
