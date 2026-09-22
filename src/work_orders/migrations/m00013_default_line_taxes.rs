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
                r#"
                CREATE TABLE IF NOT EXISTS work_orders_preferences_default_material_taxes (
                    prefs_id bigint NOT NULL REFERENCES work_orders_preferences(id) ON DELETE CASCADE,
                    tax_id bigint NOT NULL REFERENCES taxes(id) ON DELETE RESTRICT,
                    PRIMARY KEY (prefs_id, tax_id)
                );
                "#,
            )
            .await?;
            exec(
                db,
                backend,
                r#"
                CREATE TABLE IF NOT EXISTS work_orders_preferences_default_machine_taxes (
                    prefs_id bigint NOT NULL REFERENCES work_orders_preferences(id) ON DELETE CASCADE,
                    tax_id bigint NOT NULL REFERENCES taxes(id) ON DELETE RESTRICT,
                    PRIMARY KEY (prefs_id, tax_id)
                );
                "#,
            )
            .await?;
        } else {
            exec(
                db,
                backend,
                r#"
                CREATE TABLE IF NOT EXISTS work_orders_preferences_default_material_taxes (
                    prefs_id INTEGER NOT NULL REFERENCES work_orders_preferences(id) ON DELETE CASCADE,
                    tax_id INTEGER NOT NULL REFERENCES taxes(id) ON DELETE RESTRICT,
                    PRIMARY KEY (prefs_id, tax_id)
                );
                "#,
            )
            .await?;
            exec(
                db,
                backend,
                r#"
                CREATE TABLE IF NOT EXISTS work_orders_preferences_default_machine_taxes (
                    prefs_id INTEGER NOT NULL REFERENCES work_orders_preferences(id) ON DELETE CASCADE,
                    tax_id INTEGER NOT NULL REFERENCES taxes(id) ON DELETE RESTRICT,
                    PRIMARY KEY (prefs_id, tax_id)
                );
                "#,
            )
            .await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        let backend = db.get_database_backend();
        for table in [
            "work_orders_preferences_default_machine_taxes",
            "work_orders_preferences_default_material_taxes",
        ] {
            exec(db, backend, &format!("DROP TABLE IF EXISTS {table};")).await?;
        }
        Ok(())
    }
}
