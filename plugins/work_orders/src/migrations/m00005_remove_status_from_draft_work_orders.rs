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
                "DROP VIEW IF EXISTS draft_work_proposals CASCADE;".to_string(),
            ))
            .await?;
            db.execute_raw(Statement::from_string(
                backend,
                "ALTER TABLE draft_work_orders DROP COLUMN IF EXISTS status;".to_string(),
            ))
            .await?;
            db.execute_raw(Statement::from_string(
                backend,
                "CREATE OR REPLACE VIEW draft_work_proposals AS SELECT * FROM draft_work_orders;"
                    .to_string(),
            ))
            .await?;
        } else if backend == DbBackend::Sqlite {
            // SQLite 3.35.0+ supports DROP COLUMN; ignore if already absent or unsupported
            let _ = db
                .execute_raw(Statement::from_string(
                    backend,
                    "ALTER TABLE draft_work_orders DROP COLUMN status;".to_string(),
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
                "ALTER TABLE draft_work_orders ADD COLUMN IF NOT EXISTS status text NOT NULL DEFAULT 'Draft';".to_string(),
            ))
            .await?;
        }
        Ok(())
    }
}
