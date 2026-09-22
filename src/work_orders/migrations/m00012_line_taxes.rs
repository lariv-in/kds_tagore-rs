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
                CREATE TABLE IF NOT EXISTS draft_work_order_material_line_taxes (
                    line_id bigint NOT NULL REFERENCES draft_work_order_material_lines(id) ON DELETE CASCADE,
                    tax_id bigint NOT NULL REFERENCES taxes(id) ON DELETE RESTRICT,
                    PRIMARY KEY (line_id, tax_id)
                );
                "#,
            )
            .await?;
            exec(
                db,
                backend,
                r#"
                CREATE TABLE IF NOT EXISTS draft_work_order_machine_line_taxes (
                    line_id bigint NOT NULL REFERENCES draft_work_order_machine_lines(id) ON DELETE CASCADE,
                    tax_id bigint NOT NULL REFERENCES taxes(id) ON DELETE RESTRICT,
                    PRIMARY KEY (line_id, tax_id)
                );
                "#,
            )
            .await?;
            exec(
                db,
                backend,
                r#"
                CREATE TABLE IF NOT EXISTS kds_quotation_material_line_taxes (
                    line_id bigint NOT NULL REFERENCES kds_quotation_material_lines(id) ON DELETE CASCADE,
                    tax_id bigint NOT NULL REFERENCES taxes(id) ON DELETE RESTRICT,
                    PRIMARY KEY (line_id, tax_id)
                );
                "#,
            )
            .await?;
            exec(
                db,
                backend,
                r#"
                CREATE TABLE IF NOT EXISTS kds_quotation_machine_line_taxes (
                    line_id bigint NOT NULL REFERENCES kds_quotation_machine_lines(id) ON DELETE CASCADE,
                    tax_id bigint NOT NULL REFERENCES taxes(id) ON DELETE RESTRICT,
                    PRIMARY KEY (line_id, tax_id)
                );
                "#,
            )
            .await?;
        } else {
            exec(
                db,
                backend,
                r#"
                CREATE TABLE IF NOT EXISTS draft_work_order_material_line_taxes (
                    line_id INTEGER NOT NULL REFERENCES draft_work_order_material_lines(id) ON DELETE CASCADE,
                    tax_id INTEGER NOT NULL REFERENCES taxes(id) ON DELETE RESTRICT,
                    PRIMARY KEY (line_id, tax_id)
                );
                "#,
            )
            .await?;
            exec(
                db,
                backend,
                r#"
                CREATE TABLE IF NOT EXISTS draft_work_order_machine_line_taxes (
                    line_id INTEGER NOT NULL REFERENCES draft_work_order_machine_lines(id) ON DELETE CASCADE,
                    tax_id INTEGER NOT NULL REFERENCES taxes(id) ON DELETE RESTRICT,
                    PRIMARY KEY (line_id, tax_id)
                );
                "#,
            )
            .await?;
            exec(
                db,
                backend,
                r#"
                CREATE TABLE IF NOT EXISTS kds_quotation_material_line_taxes (
                    line_id INTEGER NOT NULL REFERENCES kds_quotation_material_lines(id) ON DELETE CASCADE,
                    tax_id INTEGER NOT NULL REFERENCES taxes(id) ON DELETE RESTRICT,
                    PRIMARY KEY (line_id, tax_id)
                );
                "#,
            )
            .await?;
            exec(
                db,
                backend,
                r#"
                CREATE TABLE IF NOT EXISTS kds_quotation_machine_line_taxes (
                    line_id INTEGER NOT NULL REFERENCES kds_quotation_machine_lines(id) ON DELETE CASCADE,
                    tax_id INTEGER NOT NULL REFERENCES taxes(id) ON DELETE RESTRICT,
                    PRIMARY KEY (line_id, tax_id)
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
            "kds_quotation_machine_line_taxes",
            "kds_quotation_material_line_taxes",
            "draft_work_order_machine_line_taxes",
            "draft_work_order_material_line_taxes",
        ] {
            exec(db, backend, &format!("DROP TABLE IF EXISTS {table};")).await?;
        }
        Ok(())
    }
}
