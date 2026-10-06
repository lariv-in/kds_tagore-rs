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
                "ALTER TABLE kds_quotations DROP COLUMN IF EXISTS work_order_id;".to_string(),
            ))
            .await?;
        } else if backend == DbBackend::Sqlite {
            let _ = db
                .execute_raw(Statement::from_string(
                    backend,
                    "ALTER TABLE kds_quotations DROP COLUMN work_order_id;".to_string(),
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
                "ALTER TABLE kds_quotations ADD COLUMN IF NOT EXISTS work_order_id bigint NULL;"
                    .to_string(),
            ))
            .await?;
        } else if backend == DbBackend::Sqlite {
            let _ = db
                .execute_raw(Statement::from_string(
                    backend,
                    "ALTER TABLE kds_quotations ADD COLUMN work_order_id INTEGER NULL;".to_string(),
                ))
                .await;
        }

        Ok(())
    }
}
