use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

async fn exec(db: &impl ConnectionTrait, backend: DbBackend, sql: &str) -> Result<(), DbErr> {
    db.execute_raw(Statement::from_string(backend, sql.to_string()))
        .await?;
    Ok(())
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
                "ALTER TABLE machinery_jobs ADD COLUMN IF NOT EXISTS source_doc_type text NOT NULL DEFAULT ''",
            )
            .await?;
            exec(
                db,
                backend,
                "ALTER TABLE machinery_jobs ADD COLUMN IF NOT EXISTS source_doc_id bigint NOT NULL DEFAULT 0",
            )
            .await?;
        } else {
            let _ = exec(
                db,
                backend,
                "ALTER TABLE machinery_jobs ADD COLUMN source_doc_type TEXT NOT NULL DEFAULT ''",
            )
            .await;
            let _ = exec(
                db,
                backend,
                "ALTER TABLE machinery_jobs ADD COLUMN source_doc_id INTEGER NOT NULL DEFAULT 0",
            )
            .await;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        let backend = db.get_database_backend();
        if backend == DbBackend::Postgres {
            exec(
                db,
                backend,
                "ALTER TABLE machinery_jobs DROP COLUMN IF EXISTS source_doc_id",
            )
            .await?;
            exec(
                db,
                backend,
                "ALTER TABLE machinery_jobs DROP COLUMN IF EXISTS source_doc_type",
            )
            .await?;
        } else {
            let _ = exec(
                db,
                backend,
                "ALTER TABLE machinery_jobs DROP COLUMN source_doc_id",
            )
            .await;
            let _ = exec(
                db,
                backend,
                "ALTER TABLE machinery_jobs DROP COLUMN source_doc_type",
            )
            .await;
        }
        Ok(())
    }
}
