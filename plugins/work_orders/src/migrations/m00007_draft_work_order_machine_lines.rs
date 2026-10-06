use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        let backend = db.get_database_backend();

        if backend == DbBackend::Postgres {
            db.execute_raw(Statement::from_string(
                backend,
                r#"
                CREATE TABLE IF NOT EXISTS draft_work_order_machine_lines (
                    id bigserial PRIMARY KEY,
                    created_at timestamp with time zone,
                    updated_at timestamp with time zone,
                    draft_work_order_id bigint NOT NULL REFERENCES draft_work_orders(id) ON DELETE CASCADE,
                    machine_id bigint NOT NULL REFERENCES work_order_machines(id) ON DELETE RESTRICT,
                    rate_decimal numeric(16,4) NOT NULL DEFAULT 0,
                    time_used bigint NOT NULL DEFAULT 0
                );
                "#.to_string(),
            ))
            .await?;

            db.execute_raw(Statement::from_string(
                backend,
                "CREATE INDEX IF NOT EXISTS idx_draft_work_order_machine_lines_wo ON draft_work_order_machine_lines (draft_work_order_id);".to_string(),
            ))
            .await?;
        } else if backend == DbBackend::Sqlite {
            db.execute_raw(Statement::from_string(
                backend,
                r#"
                CREATE TABLE IF NOT EXISTS draft_work_order_machine_lines (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    created_at TEXT,
                    updated_at TEXT,
                    draft_work_order_id INTEGER NOT NULL REFERENCES draft_work_orders(id) ON DELETE CASCADE,
                    machine_id INTEGER NOT NULL REFERENCES work_order_machines(id) ON DELETE RESTRICT,
                    rate_decimal NUMERIC NOT NULL DEFAULT 0,
                    time_used INTEGER NOT NULL DEFAULT 0
                );
                "#.to_string(),
            ))
            .await?;

            db.execute_raw(Statement::from_string(
                backend,
                "CREATE INDEX IF NOT EXISTS idx_draft_work_order_machine_lines_wo ON draft_work_order_machine_lines (draft_work_order_id);".to_string(),
            ))
            .await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        let backend = db.get_database_backend();

        db.execute_raw(Statement::from_string(
            backend,
            "DROP TABLE IF EXISTS draft_work_order_machine_lines;".to_string(),
        ))
        .await?;

        Ok(())
    }
}
