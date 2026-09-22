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
                "ALTER TABLE machinery_machines ADD COLUMN IF NOT EXISTS variables json NOT NULL DEFAULT '{}'",
            )
            .await?;
            exec(
                db,
                backend,
                "ALTER TABLE machinery_machines ADD COLUMN IF NOT EXISTS cost_formula text NOT NULL DEFAULT ''",
            )
            .await?;
            exec(
                db,
                backend,
                "ALTER TABLE machinery_machines DROP COLUMN IF EXISTS rate_decimal",
            )
            .await?;
        } else {
            let _ = exec(
                db,
                backend,
                "ALTER TABLE machinery_machines ADD COLUMN variables TEXT NOT NULL DEFAULT '{}'",
            )
            .await;
            let _ = exec(
                db,
                backend,
                "ALTER TABLE machinery_machines ADD COLUMN cost_formula TEXT NOT NULL DEFAULT ''",
            )
            .await;
            let _ = exec(
                db,
                backend,
                "ALTER TABLE machinery_machines DROP COLUMN rate_decimal",
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
                "ALTER TABLE machinery_machines ADD COLUMN IF NOT EXISTS rate_decimal numeric(16,4) NOT NULL DEFAULT 0",
            )
            .await?;
            exec(
                db,
                backend,
                "ALTER TABLE machinery_machines DROP COLUMN IF EXISTS variables",
            )
            .await?;
            exec(
                db,
                backend,
                "ALTER TABLE machinery_machines DROP COLUMN IF EXISTS cost_formula",
            )
            .await?;
        } else {
            let _ = exec(
                db,
                backend,
                "ALTER TABLE machinery_machines ADD COLUMN rate_decimal REAL NOT NULL DEFAULT 0",
            )
            .await;
            let _ = exec(db, backend, "ALTER TABLE machinery_machines DROP COLUMN variables")
                .await;
            let _ = exec(
                db,
                backend,
                "ALTER TABLE machinery_machines DROP COLUMN cost_formula",
            )
            .await;
        }
        Ok(())
    }
}
