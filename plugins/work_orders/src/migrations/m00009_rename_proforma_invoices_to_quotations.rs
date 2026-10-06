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
                r#"
                DO $$
                BEGIN
                    IF EXISTS (SELECT FROM pg_tables WHERE schemaname = 'public' AND tablename = 'kds_proforma_invoices')
                       AND NOT EXISTS (SELECT FROM pg_tables WHERE schemaname = 'public' AND tablename = 'kds_quotations') THEN
                        ALTER TABLE kds_proforma_invoices RENAME TO kds_quotations;
                    END IF;
                END $$;
                "#.to_string(),
            ))
            .await?;

            db.execute_raw(Statement::from_string(
                backend,
                r#"
                DO $$
                BEGIN
                    IF EXISTS (SELECT FROM pg_tables WHERE schemaname = 'public' AND tablename = 'kds_proforma_invoice_machine_lines')
                       AND NOT EXISTS (SELECT FROM pg_tables WHERE schemaname = 'public' AND tablename = 'kds_quotation_machine_lines') THEN
                        ALTER TABLE kds_proforma_invoice_machine_lines RENAME TO kds_quotation_machine_lines;
                    END IF;
                END $$;
                "#.to_string(),
            ))
            .await?;

            db.execute_raw(Statement::from_string(
                backend,
                r#"
                DO $$
                BEGIN
                    IF EXISTS (SELECT FROM pg_tables WHERE schemaname = 'public' AND tablename = 'kds_proforma_invoice_material_lines')
                       AND NOT EXISTS (SELECT FROM pg_tables WHERE schemaname = 'public' AND tablename = 'kds_quotation_material_lines') THEN
                        ALTER TABLE kds_proforma_invoice_material_lines RENAME TO kds_quotation_material_lines;
                    END IF;
                END $$;
                "#.to_string(),
            ))
            .await?;

            db.execute_raw(Statement::from_string(
                backend,
                "ALTER TABLE work_orders_preferences RENAME COLUMN proforma_invoice_pdf_template TO quotation_pdf_template;".to_string(),
            ))
            .await?;
        } else if backend == DbBackend::Sqlite {
            let _ = db
                .execute_raw(Statement::from_string(
                    backend,
                    "ALTER TABLE kds_proforma_invoices RENAME TO kds_quotations;".to_string(),
                ))
                .await;
            let _ = db
                .execute_raw(Statement::from_string(
                    backend,
                    "ALTER TABLE kds_proforma_invoice_machine_lines RENAME TO kds_quotation_machine_lines;"
                        .to_string(),
                ))
                .await;
            let _ = db
                .execute_raw(Statement::from_string(
                    backend,
                    "ALTER TABLE kds_proforma_invoice_material_lines RENAME TO kds_quotation_material_lines;"
                        .to_string(),
                ))
                .await;
            let _ = db
                .execute_raw(Statement::from_string(
                    backend,
                    "ALTER TABLE work_orders_preferences RENAME COLUMN proforma_invoice_pdf_template TO quotation_pdf_template;"
                        .to_string(),
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
                "ALTER TABLE work_orders_preferences RENAME COLUMN quotation_pdf_template TO proforma_invoice_pdf_template;".to_string(),
            ))
            .await?;

            db.execute_raw(Statement::from_string(
                backend,
                r#"
                DO $$
                BEGIN
                    IF EXISTS (SELECT FROM pg_tables WHERE schemaname = 'public' AND tablename = 'kds_quotation_material_lines')
                       AND NOT EXISTS (SELECT FROM pg_tables WHERE schemaname = 'public' AND tablename = 'kds_proforma_invoice_material_lines') THEN
                        ALTER TABLE kds_quotation_material_lines RENAME TO kds_proforma_invoice_material_lines;
                    END IF;
                END $$;
                "#.to_string(),
            ))
            .await?;

            db.execute_raw(Statement::from_string(
                backend,
                r#"
                DO $$
                BEGIN
                    IF EXISTS (SELECT FROM pg_tables WHERE schemaname = 'public' AND tablename = 'kds_quotation_machine_lines')
                       AND NOT EXISTS (SELECT FROM pg_tables WHERE schemaname = 'public' AND tablename = 'kds_proforma_invoice_machine_lines') THEN
                        ALTER TABLE kds_quotation_machine_lines RENAME TO kds_proforma_invoice_machine_lines;
                    END IF;
                END $$;
                "#.to_string(),
            ))
            .await?;

            db.execute_raw(Statement::from_string(
                backend,
                r#"
                DO $$
                BEGIN
                    IF EXISTS (SELECT FROM pg_tables WHERE schemaname = 'public' AND tablename = 'kds_quotations')
                       AND NOT EXISTS (SELECT FROM pg_tables WHERE schemaname = 'public' AND tablename = 'kds_proforma_invoices') THEN
                        ALTER TABLE kds_quotations RENAME TO kds_proforma_invoices;
                    END IF;
                END $$;
                "#.to_string(),
            ))
            .await?;
        } else if backend == DbBackend::Sqlite {
            let _ = db
                .execute_raw(Statement::from_string(
                    backend,
                    "ALTER TABLE work_orders_preferences RENAME COLUMN quotation_pdf_template TO proforma_invoice_pdf_template;"
                        .to_string(),
                ))
                .await;
            let _ = db
                .execute_raw(Statement::from_string(
                    backend,
                    "ALTER TABLE kds_quotation_material_lines RENAME TO kds_proforma_invoice_material_lines;"
                        .to_string(),
                ))
                .await;
            let _ = db
                .execute_raw(Statement::from_string(
                    backend,
                    "ALTER TABLE kds_quotation_machine_lines RENAME TO kds_proforma_invoice_machine_lines;"
                        .to_string(),
                ))
                .await;
            let _ = db
                .execute_raw(Statement::from_string(
                    backend,
                    "ALTER TABLE kds_quotations RENAME TO kds_proforma_invoices;".to_string(),
                ))
                .await;
        }

        Ok(())
    }
}
