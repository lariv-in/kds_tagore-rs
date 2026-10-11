//! Free-text remarks on quotation material and machine lines.

use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        let backend = db.get_database_backend();

        for table in [
            "kds_quotation_material_lines",
            "kds_quotation_machine_lines",
        ] {
            if backend == DbBackend::Postgres {
                db.execute_raw(Statement::from_string(
                    backend,
                    format!(
                        "ALTER TABLE {table} ADD COLUMN IF NOT EXISTS remarks text NOT NULL DEFAULT '';"
                    ),
                ))
                .await?;
            } else if backend == DbBackend::Sqlite {
                let _ = db
                    .execute_raw(Statement::from_string(
                        backend,
                        format!("ALTER TABLE {table} ADD COLUMN remarks TEXT NOT NULL DEFAULT '';"),
                    ))
                    .await;
            }
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        let backend = db.get_database_backend();

        for table in [
            "kds_quotation_material_lines",
            "kds_quotation_machine_lines",
        ] {
            if backend == DbBackend::Postgres {
                db.execute_raw(Statement::from_string(
                    backend,
                    format!("ALTER TABLE {table} DROP COLUMN IF EXISTS remarks;"),
                ))
                .await?;
            } else if backend == DbBackend::Sqlite {
                let _ = db
                    .execute_raw(Statement::from_string(
                        backend,
                        format!("ALTER TABLE {table} DROP COLUMN remarks;"),
                    ))
                    .await;
            }
        }

        Ok(())
    }
}
