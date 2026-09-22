//! Quotations no longer store job duration; it is set when creating a work order.

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
                "ALTER TABLE kds_quotations DROP COLUMN IF EXISTS duration;".to_string(),
            ))
            .await?;
        } else if backend == DbBackend::Sqlite {
            let _ = db
                .execute_raw(Statement::from_string(
                    backend,
                    "ALTER TABLE kds_quotations DROP COLUMN duration;".to_string(),
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
                "ALTER TABLE kds_quotations ADD COLUMN IF NOT EXISTS duration bigint NOT NULL DEFAULT 0;"
                    .to_string(),
            ))
            .await?;
        } else if backend == DbBackend::Sqlite {
            let _ = db
                .execute_raw(Statement::from_string(
                    backend,
                    "ALTER TABLE kds_quotations ADD COLUMN duration INTEGER NOT NULL DEFAULT 0;"
                        .to_string(),
                ))
                .await;
        }

        Ok(())
    }
}
