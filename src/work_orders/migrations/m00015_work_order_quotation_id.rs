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
                ALTER TABLE draft_work_orders
                    ADD COLUMN IF NOT EXISTS quotation_id bigint NULL
                    REFERENCES kds_quotations(id) ON DELETE SET NULL;
                "#
                .to_string(),
            ))
            .await?;
            db.execute_raw(Statement::from_string(
                backend,
                "CREATE INDEX IF NOT EXISTS idx_draft_work_orders_quotation ON draft_work_orders (quotation_id);"
                    .to_string(),
            ))
            .await?;
        } else if backend == DbBackend::Sqlite {
            let _ = db
                .execute_raw(Statement::from_string(
                    backend,
                    "ALTER TABLE draft_work_orders ADD COLUMN quotation_id INTEGER NULL REFERENCES kds_quotations(id) ON DELETE SET NULL;"
                        .to_string(),
                ))
                .await;
            let _ = db
                .execute_raw(Statement::from_string(
                    backend,
                    "CREATE INDEX IF NOT EXISTS idx_draft_work_orders_quotation ON draft_work_orders (quotation_id);"
                        .to_string(),
                ))
                .await;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        let backend = db.get_database_backend();

        if backend == DbBackend::Postgres {
            db.execute_raw(Statement::from_string(
                backend,
                "DROP INDEX IF EXISTS idx_draft_work_orders_quotation;".to_string(),
            ))
            .await?;
            db.execute_raw(Statement::from_string(
                backend,
                "ALTER TABLE draft_work_orders DROP COLUMN IF EXISTS quotation_id;".to_string(),
            ))
            .await?;
        } else if backend == DbBackend::Sqlite {
            let _ = db
                .execute_raw(Statement::from_string(
                    backend,
                    "DROP INDEX IF EXISTS idx_draft_work_orders_quotation;".to_string(),
                ))
                .await;
            let _ = db
                .execute_raw(Statement::from_string(
                    backend,
                    "ALTER TABLE draft_work_orders DROP COLUMN quotation_id;".to_string(),
                ))
                .await;
        }

        Ok(())
    }
}
