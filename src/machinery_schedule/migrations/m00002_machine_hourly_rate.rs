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
                "ALTER TABLE machinery_machines ADD COLUMN IF NOT EXISTS rate_decimal numeric(16,4) NOT NULL DEFAULT 0;".to_string(),
            ))
            .await?;
        } else {
            let _ = db
                .execute_raw(Statement::from_string(
                    backend,
                    "ALTER TABLE machinery_machines ADD COLUMN rate_decimal REAL NOT NULL DEFAULT 0;".to_string(),
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
                "ALTER TABLE machinery_machines DROP COLUMN IF EXISTS rate_decimal;".to_string(),
            ))
            .await?;
        } else {
            let _ = db
                .execute_raw(Statement::from_string(
                    backend,
                    "ALTER TABLE machinery_machines DROP COLUMN rate_decimal;".to_string(),
                ))
                .await;
        }

        Ok(())
    }
}
