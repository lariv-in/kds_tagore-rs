use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

async fn exec(db: &impl ConnectionTrait, backend: DbBackend, sql: &str) -> Result<(), DbErr> {
    db.execute_raw(Statement::from_string(backend, sql.to_string()))
        .await?;
    Ok(())
}

async fn exec_ok(db: &impl ConnectionTrait, backend: DbBackend, sql: &str) {
    let _ = exec(db, backend, sql).await;
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        let backend = db.get_database_backend();

        if backend == DbBackend::Postgres {
            exec(
                db,
                backend,
                "DROP VIEW IF EXISTS draft_work_order_lines CASCADE",
            )
            .await?;
            exec(
                db,
                backend,
                "DROP VIEW IF EXISTS draft_work_proposal_lines CASCADE",
            )
            .await?;
            exec(db, backend, "DROP VIEW IF EXISTS work_order_lines CASCADE").await?;

            exec(
                db,
                backend,
                "ALTER TABLE work_order_components ADD COLUMN IF NOT EXISTS cost_formula text NOT NULL DEFAULT ''",
            )
            .await?;
            exec(
                db,
                backend,
                "ALTER TABLE work_order_components ADD COLUMN IF NOT EXISTS weight_formula text NOT NULL DEFAULT ''",
            )
            .await?;
            exec(
                db,
                backend,
                "ALTER TABLE work_order_components ADD COLUMN IF NOT EXISTS variables json NOT NULL DEFAULT '{}'",
            )
            .await?;
            exec(
                db,
                backend,
                "ALTER TABLE work_order_components DROP COLUMN IF EXISTS shape_id CASCADE",
            )
            .await?;
            exec(
                db,
                backend,
                "ALTER TABLE work_order_components DROP COLUMN IF EXISTS material_id CASCADE",
            )
            .await?;
            exec(
                db,
                backend,
                "ALTER TABLE work_order_components DROP COLUMN IF EXISTS fixed_variables CASCADE",
            )
            .await?;
            for table in [
                "draft_work_order_material_lines",
                "kds_quotation_material_lines",
                "kds_work_order_material_lines",
            ] {
                exec(
                    db,
                    backend,
                    &format!("ALTER TABLE {table} DROP COLUMN IF EXISTS quantity CASCADE"),
                )
                .await?;
                exec(
                    db,
                    backend,
                    &format!("ALTER TABLE {table} DROP COLUMN IF EXISTS unit_weight CASCADE"),
                )
                .await?;
                exec(
                    db,
                    backend,
                    &format!("ALTER TABLE {table} DROP COLUMN IF EXISTS material_rate CASCADE"),
                )
                .await?;
            }
            exec(
                db,
                backend,
                "CREATE OR REPLACE VIEW draft_work_order_lines AS SELECT * FROM draft_work_order_material_lines",
            )
            .await?;
            exec(
                db,
                backend,
                "CREATE OR REPLACE VIEW draft_work_proposal_lines AS SELECT * FROM draft_work_order_material_lines",
            )
            .await?;
            exec(
                db,
                backend,
                "CREATE OR REPLACE VIEW work_order_lines AS SELECT * FROM draft_work_order_material_lines",
            )
            .await?;
            for table in [
                "draft_work_order_machine_lines",
                "kds_quotation_machine_lines",
                "kds_work_order_machine_lines",
            ] {
                exec(
                    db,
                    backend,
                    &format!(
                        "ALTER TABLE {table} ADD COLUMN IF NOT EXISTS variables json NOT NULL DEFAULT '{{}}'"
                    ),
                )
                .await?;
                exec(
                    db,
                    backend,
                    &format!(
                        "ALTER TABLE {table} ADD COLUMN IF NOT EXISTS final_cost numeric(19,6) NOT NULL DEFAULT 0"
                    ),
                )
                .await?;
                exec(
                    db,
                    backend,
                    &format!("ALTER TABLE {table} DROP COLUMN IF EXISTS rate_decimal CASCADE"),
                )
                .await?;
                exec(
                    db,
                    backend,
                    &format!("ALTER TABLE {table} DROP COLUMN IF EXISTS time_used CASCADE"),
                )
                .await?;
            }
            exec(
                db,
                backend,
                "ALTER TABLE draft_work_orders ADD COLUMN IF NOT EXISTS duration bigint NOT NULL DEFAULT 0",
            )
            .await?;
            exec(
                db,
                backend,
                "ALTER TABLE kds_quotations ADD COLUMN IF NOT EXISTS duration bigint NOT NULL DEFAULT 0",
            )
            .await?;
            exec(
                db,
                backend,
                "ALTER TABLE kds_work_orders ADD COLUMN IF NOT EXISTS duration bigint NOT NULL DEFAULT 0",
            )
            .await?;
            exec(
                db,
                backend,
                "DROP TABLE IF EXISTS work_order_material_rates CASCADE",
            )
            .await?;
            exec(
                db,
                backend,
                "DROP TABLE IF EXISTS work_order_materials CASCADE",
            )
            .await?;
            exec(
                db,
                backend,
                "DROP TABLE IF EXISTS work_order_shapes CASCADE",
            )
            .await?;
        } else {
            let _ = exec(db, backend, "PRAGMA foreign_keys = OFF").await;
            exec_ok(db, backend, "DROP VIEW IF EXISTS draft_work_order_lines").await;
            exec_ok(db, backend, "DROP VIEW IF EXISTS draft_work_proposal_lines").await;
            exec_ok(db, backend, "DROP VIEW IF EXISTS work_order_lines").await;

            exec(
                db,
                backend,
                r#"
                CREATE TABLE work_order_components_new (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    created_at TEXT,
                    updated_at TEXT,
                    name TEXT NOT NULL,
                    cost_formula TEXT NOT NULL DEFAULT '',
                    weight_formula TEXT NOT NULL DEFAULT '',
                    variables TEXT NOT NULL DEFAULT '{}'
                )
                "#,
            )
            .await?;
            exec(
                db,
                backend,
                r#"
                INSERT INTO work_order_components_new (id, created_at, updated_at, name)
                SELECT id, created_at, updated_at, name FROM work_order_components
                "#,
            )
            .await?;
            exec(db, backend, "DROP TABLE work_order_components").await?;
            exec(
                db,
                backend,
                "ALTER TABLE work_order_components_new RENAME TO work_order_components",
            )
            .await?;

            exec(
                db,
                backend,
                r#"
                CREATE TABLE draft_work_order_material_lines_new (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    created_at TEXT,
                    updated_at TEXT,
                    draft_work_order_id INTEGER NOT NULL REFERENCES draft_work_orders(id) ON DELETE CASCADE,
                    component_id INTEGER NOT NULL REFERENCES work_order_components(id) ON DELETE RESTRICT,
                    variables TEXT NOT NULL DEFAULT '{}',
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
                INSERT INTO draft_work_order_material_lines_new
                    (id, created_at, updated_at, draft_work_order_id, component_id, variables, final_cost, extra_data)
                SELECT id, created_at, updated_at, draft_work_order_id, component_id, variables, final_cost, extra_data
                FROM draft_work_order_material_lines
                "#,
            )
            .await?;
            exec(db, backend, "DROP TABLE draft_work_order_material_lines").await?;
            exec(
                db,
                backend,
                "ALTER TABLE draft_work_order_material_lines_new RENAME TO draft_work_order_material_lines",
            )
            .await?;

            exec(
                db,
                backend,
                r#"
                CREATE TABLE kds_quotation_material_lines_new (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    created_at TEXT,
                    updated_at TEXT,
                    invoice_id INTEGER NOT NULL REFERENCES kds_quotations(id) ON DELETE CASCADE,
                    component_id INTEGER NOT NULL REFERENCES work_order_components(id) ON DELETE RESTRICT,
                    variables TEXT NOT NULL DEFAULT '{}',
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
                INSERT INTO kds_quotation_material_lines_new
                    (id, created_at, updated_at, invoice_id, component_id, variables, final_cost, extra_data)
                SELECT id, created_at, updated_at, invoice_id, component_id, variables, final_cost, extra_data
                FROM kds_quotation_material_lines
                "#,
            )
            .await?;
            exec(db, backend, "DROP TABLE kds_quotation_material_lines").await?;
            exec(
                db,
                backend,
                "ALTER TABLE kds_quotation_material_lines_new RENAME TO kds_quotation_material_lines",
            )
            .await?;

            exec(
                db,
                backend,
                r#"
                CREATE TABLE kds_work_order_material_lines_new (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    created_at TEXT,
                    updated_at TEXT,
                    work_order_id INTEGER NOT NULL REFERENCES kds_work_orders(id) ON DELETE CASCADE,
                    component_id INTEGER NOT NULL REFERENCES work_order_components(id) ON DELETE RESTRICT,
                    variables TEXT NOT NULL DEFAULT '{}',
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
                INSERT INTO kds_work_order_material_lines_new
                    (id, created_at, updated_at, work_order_id, component_id, variables, final_cost, extra_data)
                SELECT id, created_at, updated_at, work_order_id, component_id, variables, final_cost, extra_data
                FROM kds_work_order_material_lines
                "#,
            )
            .await?;
            exec(db, backend, "DROP TABLE kds_work_order_material_lines").await?;
            exec(
                db,
                backend,
                "ALTER TABLE kds_work_order_material_lines_new RENAME TO kds_work_order_material_lines",
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
                    machine_id INTEGER NOT NULL REFERENCES machinery_machines(id) ON DELETE RESTRICT,
                    variables TEXT NOT NULL DEFAULT '{}',
                    final_cost REAL NOT NULL DEFAULT 0
                )
                "#,
            )
            .await?;
            exec(
                db,
                backend,
                r#"
                INSERT INTO draft_work_order_machine_lines_new
                    (id, created_at, updated_at, draft_work_order_id, machine_id)
                SELECT id, created_at, updated_at, draft_work_order_id, machine_id
                FROM draft_work_order_machine_lines
                "#,
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
                r#"
                CREATE TABLE kds_quotation_machine_lines_new (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    created_at TEXT,
                    updated_at TEXT,
                    invoice_id INTEGER NOT NULL REFERENCES kds_quotations(id) ON DELETE CASCADE,
                    machine_id INTEGER NULL REFERENCES machinery_machines(id) ON DELETE SET NULL,
                    name TEXT NOT NULL,
                    variables TEXT NOT NULL DEFAULT '{}',
                    final_cost REAL NOT NULL DEFAULT 0
                )
                "#,
            )
            .await?;
            exec(
                db,
                backend,
                r#"
                INSERT INTO kds_quotation_machine_lines_new
                    (id, created_at, updated_at, invoice_id, machine_id, name)
                SELECT id, created_at, updated_at, invoice_id, machine_id, name
                FROM kds_quotation_machine_lines
                "#,
            )
            .await?;
            exec(db, backend, "DROP TABLE kds_quotation_machine_lines").await?;
            exec(
                db,
                backend,
                "ALTER TABLE kds_quotation_machine_lines_new RENAME TO kds_quotation_machine_lines",
            )
            .await?;

            exec(
                db,
                backend,
                r#"
                CREATE TABLE kds_work_order_machine_lines_new (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    created_at TEXT,
                    updated_at TEXT,
                    work_order_id INTEGER NOT NULL REFERENCES kds_work_orders(id) ON DELETE CASCADE,
                    machine_id INTEGER NULL REFERENCES machinery_machines(id) ON DELETE SET NULL,
                    name TEXT NOT NULL,
                    variables TEXT NOT NULL DEFAULT '{}',
                    final_cost REAL NOT NULL DEFAULT 0
                )
                "#,
            )
            .await?;
            exec(
                db,
                backend,
                r#"
                INSERT INTO kds_work_order_machine_lines_new
                    (id, created_at, updated_at, work_order_id, machine_id, name)
                SELECT id, created_at, updated_at, work_order_id, machine_id, name
                FROM kds_work_order_machine_lines
                "#,
            )
            .await?;
            exec(db, backend, "DROP TABLE kds_work_order_machine_lines").await?;
            exec(
                db,
                backend,
                "ALTER TABLE kds_work_order_machine_lines_new RENAME TO kds_work_order_machine_lines",
            )
            .await?;

            exec_ok(
                db,
                backend,
                "ALTER TABLE draft_work_orders ADD COLUMN duration INTEGER NOT NULL DEFAULT 0",
            )
            .await;
            exec_ok(
                db,
                backend,
                "ALTER TABLE kds_quotations ADD COLUMN duration INTEGER NOT NULL DEFAULT 0",
            )
            .await;
            exec_ok(
                db,
                backend,
                "ALTER TABLE kds_work_orders ADD COLUMN duration INTEGER NOT NULL DEFAULT 0",
            )
            .await;
            exec_ok(
                db,
                backend,
                "DROP TABLE IF EXISTS work_order_material_rates",
            )
            .await;
            exec_ok(db, backend, "DROP TABLE IF EXISTS work_order_materials").await;
            exec_ok(db, backend, "DROP TABLE IF EXISTS work_order_shapes").await;
            let _ = exec(db, backend, "PRAGMA foreign_keys = ON").await;
        }

        crate::work_orders::seed::ensure_standard_seeds(db).await
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
