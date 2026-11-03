//! Replace costing work orders with a process plan.
//!
//! Issued jobs, drafts, components, and preferences are dropped. Existing rows
//! are not converted. Machinery jobs that pointed at a work order are detached.

use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const DROP_TABLES: &[&str] = &[
    "draft_work_order_material_line_taxes",
    "draft_work_order_machine_line_taxes",
    "kds_work_order_material_line_taxes",
    "kds_work_order_machine_line_taxes",
    "kds_quotation_material_line_taxes",
    "kds_quotation_machine_line_taxes",
    "work_orders_preferences_default_material_taxes",
    "work_orders_preferences_default_machine_taxes",
    "draft_work_order_material_lines",
    "draft_work_order_machine_lines",
    "draft_work_order_lines",
    "draft_work_proposal_lines",
    "kds_work_order_material_lines",
    "kds_work_order_machine_lines",
    "kds_quotation_material_lines",
    "kds_quotation_machine_lines",
    "work_order_lines",
    "draft_work_orders",
    "work_order_components",
    "work_orders_preferences",
    "kds_work_orders",
];

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

async fn column_exists(
    db: &impl ConnectionTrait,
    backend: DbBackend,
    table: &str,
    column: &str,
) -> Result<bool, DbErr> {
    if !table_exists(db, backend, table).await? {
        return Ok(false);
    }
    if backend == DbBackend::Postgres {
        let sql = format!(
            "SELECT COUNT(*) AS n FROM information_schema.columns WHERE table_schema = 'public' AND table_name = '{table}' AND column_name = '{column}'"
        );
        let row = db
            .query_one_raw(Statement::from_string(backend, sql))
            .await?;
        let n: i64 = row.map(|r| r.try_get("", "n").unwrap_or(0)).unwrap_or(0);
        Ok(n > 0)
    } else {
        let sql =
            format!("SELECT 1 AS n FROM pragma_table_info('{table}') WHERE name = '{column}'");
        let row = db
            .query_one_raw(Statement::from_string(backend, sql))
            .await?;
        Ok(row.is_some())
    }
}

fn fk(name: &str, column: &str, table: &str) -> String {
    format!(
        "CONSTRAINT {name} FOREIGN KEY ({column}) REFERENCES {table}(id) ON UPDATE CASCADE ON DELETE RESTRICT"
    )
}

fn create_sql(postgres: bool, customers: bool, nodes: bool, users: bool) -> String {
    let (id, ts, qty) = if postgres {
        (
            "bigserial PRIMARY KEY",
            "timestamp with time zone",
            "numeric(19, 6)",
        )
    } else {
        ("integer PRIMARY KEY AUTOINCREMENT", "text", "real(19, 6)")
    };
    let customer_fk = if customers {
        format!(", {}", fk("fk_kds_wo_customer", "customer_id", "customers"))
    } else {
        String::new()
    };
    let folder_fk = if nodes {
        format!(
            ", {}",
            fk("fk_kds_wo_folder", "folder_vnode_id", "filesystem_nodes")
        )
    } else {
        String::new()
    };
    let user_fk = if users {
        format!(", {}", fk("fk_kds_wo_done_user", "completed_by", "users"))
    } else {
        String::new()
    };
    format!(
        r#"
        CREATE TABLE kds_work_orders (
            id {id},
            created_at {ts},
            updated_at {ts},
            title text NOT NULL,
            customer_id bigint NOT NULL,
            folder_vnode_id bigint NOT NULL
            {customer_fk}
            {folder_fk}
        );
        CREATE TABLE kds_work_order_parts (
            id {id},
            created_at {ts},
            updated_at {ts},
            work_order_id bigint NOT NULL,
            name text NOT NULL,
            measure_kind text NOT NULL,
            measure_unit text NOT NULL,
            CONSTRAINT fk_kds_wo_parts_order FOREIGN KEY (work_order_id) REFERENCES kds_work_orders(id) ON UPDATE CASCADE ON DELETE CASCADE,
            CONSTRAINT ux_kds_wo_parts_name UNIQUE (work_order_id, name)
        );
        CREATE TABLE kds_work_order_presupplied_parts (
            id {id},
            created_at {ts},
            updated_at {ts},
            work_order_id bigint NOT NULL,
            part_id bigint NOT NULL,
            qty_value {qty} NOT NULL,
            qty_unit text NOT NULL,
            CONSTRAINT fk_kds_wo_pre_order FOREIGN KEY (work_order_id) REFERENCES kds_work_orders(id) ON UPDATE CASCADE ON DELETE CASCADE,
            CONSTRAINT fk_kds_wo_pre_part FOREIGN KEY (part_id) REFERENCES kds_work_order_parts(id) ON UPDATE CASCADE ON DELETE RESTRICT,
            CONSTRAINT ux_kds_wo_presupplied UNIQUE (work_order_id, part_id)
        );
        CREATE TABLE kds_work_order_steps (
            id {id},
            created_at {ts},
            updated_at {ts},
            work_order_id bigint NOT NULL,
            name text NOT NULL,
            planned_batches integer NOT NULL,
            CONSTRAINT fk_kds_wo_steps_order FOREIGN KEY (work_order_id) REFERENCES kds_work_orders(id) ON UPDATE CASCADE ON DELETE CASCADE,
            CONSTRAINT ux_kds_wo_steps_name UNIQUE (work_order_id, name)
        );
        CREATE TABLE kds_work_order_step_inputs (
            id {id},
            created_at {ts},
            updated_at {ts},
            step_id bigint NOT NULL,
            part_id bigint NOT NULL,
            qty_value {qty} NOT NULL,
            qty_unit text NOT NULL,
            CONSTRAINT fk_kds_wo_in_step FOREIGN KEY (step_id) REFERENCES kds_work_order_steps(id) ON UPDATE CASCADE ON DELETE CASCADE,
            CONSTRAINT fk_kds_wo_in_part FOREIGN KEY (part_id) REFERENCES kds_work_order_parts(id) ON UPDATE CASCADE ON DELETE RESTRICT,
            CONSTRAINT ux_kds_wo_step_inputs UNIQUE (step_id, part_id)
        );
        CREATE TABLE kds_work_order_step_outputs (
            id {id},
            created_at {ts},
            updated_at {ts},
            step_id bigint NOT NULL,
            part_id bigint NOT NULL,
            qty_value {qty} NOT NULL,
            qty_unit text NOT NULL,
            CONSTRAINT fk_kds_wo_out_step FOREIGN KEY (step_id) REFERENCES kds_work_order_steps(id) ON UPDATE CASCADE ON DELETE CASCADE,
            CONSTRAINT fk_kds_wo_out_part FOREIGN KEY (part_id) REFERENCES kds_work_order_parts(id) ON UPDATE CASCADE ON DELETE RESTRICT,
            CONSTRAINT ux_kds_wo_step_outputs UNIQUE (step_id, part_id)
        );
        CREATE TABLE kds_work_order_completed_steps (
            id {id},
            created_at {ts},
            updated_at {ts},
            step_id bigint NOT NULL,
            completed_at {ts} NOT NULL,
            completed_by bigint NOT NULL,
            CONSTRAINT fk_kds_wo_done_step FOREIGN KEY (step_id) REFERENCES kds_work_order_steps(id) ON UPDATE CASCADE ON DELETE RESTRICT
            {user_fk}
        );
        CREATE INDEX idx_kds_wo_done_step ON kds_work_order_completed_steps (step_id);
        CREATE TABLE kds_work_order_completed_step_inputs (
            id {id},
            created_at {ts},
            updated_at {ts},
            completed_step_id bigint NOT NULL,
            part_id bigint NOT NULL,
            qty_value {qty} NOT NULL,
            qty_unit text NOT NULL,
            CONSTRAINT fk_kds_wo_done_in FOREIGN KEY (completed_step_id) REFERENCES kds_work_order_completed_steps(id) ON UPDATE CASCADE ON DELETE CASCADE,
            CONSTRAINT fk_kds_wo_done_in_part FOREIGN KEY (part_id) REFERENCES kds_work_order_parts(id) ON UPDATE CASCADE ON DELETE RESTRICT,
            CONSTRAINT ux_kds_wo_done_inputs UNIQUE (completed_step_id, part_id)
        );
        CREATE TABLE kds_work_order_completed_step_outputs (
            id {id},
            created_at {ts},
            updated_at {ts},
            completed_step_id bigint NOT NULL,
            part_id bigint NOT NULL,
            qty_value {qty} NOT NULL,
            qty_unit text NOT NULL,
            CONSTRAINT fk_kds_wo_done_out FOREIGN KEY (completed_step_id) REFERENCES kds_work_order_completed_steps(id) ON UPDATE CASCADE ON DELETE CASCADE,
            CONSTRAINT fk_kds_wo_done_out_part FOREIGN KEY (part_id) REFERENCES kds_work_order_parts(id) ON UPDATE CASCADE ON DELETE RESTRICT,
            CONSTRAINT ux_kds_wo_done_outputs UNIQUE (completed_step_id, part_id)
        );
        "#
    )
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        let backend = db.get_database_backend();

        if column_exists(db, backend, "machinery_jobs", "source_doc_type").await?
            && column_exists(db, backend, "machinery_jobs", "source_doc_id").await?
        {
            exec(
                db,
                backend,
                "UPDATE machinery_jobs SET source_doc_type = '', source_doc_id = 0 WHERE source_doc_type = 'kds_wo.WorkOrder'",
            )
            .await?;
        }

        for table in DROP_TABLES {
            let sql = if backend == DbBackend::Postgres {
                format!("DROP TABLE IF EXISTS {table} CASCADE")
            } else {
                format!("DROP TABLE IF EXISTS {table}")
            };
            exec(db, backend, &sql).await?;
        }

        let sql = create_sql(
            backend == DbBackend::Postgres,
            table_exists(db, backend, "customers").await?,
            table_exists(db, backend, "filesystem_nodes").await?,
            table_exists(db, backend, "users").await?,
        );
        for statement in sql.split(';') {
            let statement = statement.trim();
            if !statement.is_empty() {
                exec(db, backend, statement).await?;
            }
        }
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
