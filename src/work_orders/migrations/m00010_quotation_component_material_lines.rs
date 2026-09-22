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
                "DROP TABLE IF EXISTS kds_quotation_material_lines CASCADE;".to_string(),
            ))
            .await?;

            db.execute_raw(Statement::from_string(
                backend,
                r#"
                CREATE TABLE kds_quotation_material_lines (
                    id bigserial PRIMARY KEY,
                    created_at timestamp with time zone,
                    updated_at timestamp with time zone,
                    invoice_id bigint NOT NULL REFERENCES kds_quotations(id) ON DELETE CASCADE,
                    component_id bigint NOT NULL REFERENCES work_order_components(id) ON DELETE RESTRICT,
                    variables json NOT NULL DEFAULT '{}',
                    quantity numeric(16,4) NOT NULL DEFAULT 1,
                    unit_weight numeric(16,4) NOT NULL DEFAULT 0,
                    material_rate numeric(16,4) NOT NULL DEFAULT 0,
                    final_cost numeric(16,4) NOT NULL DEFAULT 0,
                    extra_data json NOT NULL DEFAULT '{}'
                );
                "#
                .to_string(),
            ))
            .await?;
        } else if backend == DbBackend::Sqlite {
            db.execute_raw(Statement::from_string(
                backend,
                "DROP TABLE IF EXISTS kds_quotation_material_lines;".to_string(),
            ))
            .await?;

            db.execute_raw(Statement::from_string(
                backend,
                r#"
                CREATE TABLE kds_quotation_material_lines (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    created_at TEXT,
                    updated_at TEXT,
                    invoice_id INTEGER NOT NULL REFERENCES kds_quotations(id) ON DELETE CASCADE,
                    component_id INTEGER NOT NULL REFERENCES work_order_components(id) ON DELETE RESTRICT,
                    variables TEXT NOT NULL DEFAULT '{}',
                    quantity REAL NOT NULL DEFAULT 1,
                    unit_weight REAL NOT NULL DEFAULT 0,
                    material_rate REAL NOT NULL DEFAULT 0,
                    final_cost REAL NOT NULL DEFAULT 0,
                    extra_data TEXT NOT NULL DEFAULT '{}'
                );
                "#
                .to_string(),
            ))
            .await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        let backend = db.get_database_backend();

        if backend == DbBackend::Postgres {
            db.execute_raw(Statement::from_string(
                backend,
                "DROP TABLE IF EXISTS kds_quotation_material_lines CASCADE;".to_string(),
            ))
            .await?;

            db.execute_raw(Statement::from_string(
                backend,
                r#"
                CREATE TABLE kds_quotation_material_lines (
                    id bigserial PRIMARY KEY,
                    created_at timestamp with time zone,
                    updated_at timestamp with time zone,
                    invoice_id bigint NOT NULL REFERENCES kds_quotations(id) ON DELETE CASCADE,
                    material_id bigint NULL,
                    name text NOT NULL,
                    rate_decimal numeric(16,4) NOT NULL,
                    qty_decimal numeric(16,4) NOT NULL
                );
                "#
                .to_string(),
            ))
            .await?;
        } else if backend == DbBackend::Sqlite {
            db.execute_raw(Statement::from_string(
                backend,
                "DROP TABLE IF EXISTS kds_quotation_material_lines;".to_string(),
            ))
            .await?;

            db.execute_raw(Statement::from_string(
                backend,
                r#"
                CREATE TABLE kds_quotation_material_lines (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    created_at TEXT,
                    updated_at TEXT,
                    invoice_id INTEGER NOT NULL REFERENCES kds_quotations(id) ON DELETE CASCADE,
                    material_id INTEGER NULL,
                    name TEXT NOT NULL,
                    rate_decimal REAL NOT NULL,
                    qty_decimal REAL NOT NULL
                );
                "#
                .to_string(),
            ))
            .await?;
        }

        Ok(())
    }
}
