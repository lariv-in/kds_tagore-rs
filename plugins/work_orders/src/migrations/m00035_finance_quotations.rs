//! Copy `kds_quotations` into finance `quotations`, then drop the KDS tables.
//!
//! Quotation ids are preserved so existing work-order `quotation_id` values stay valid.
//! Material lines become goods lines and machine lines become service lines, each with
//! quantity 1 and rate equal to the stored final cost. Line variable values are copied
//! as stored; earlier migrations have already rewritten them into formula measures.

use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const UP: &str = r#"
DO $$
DECLARE
    rec record;
    new_id bigint;
    con record;
BEGIN
    IF to_regclass('public.kds_quotations') IS NULL
        OR to_regclass('public.quotations') IS NULL THEN
        RETURN;
    END IF;

    INSERT INTO customers (created_at, updated_at, is_individual, company_id, individual_id)
    SELECT NOW(), NOW(), false, company.id, NULL
    FROM (SELECT DISTINCT customer_id AS id FROM kds_quotations) company
    WHERE company.id IS NOT NULL
      AND NOT EXISTS (
          SELECT 1 FROM customers existing
          WHERE existing.is_individual = false
            AND existing.company_id = company.id
      );

    INSERT INTO quotations (
        id, created_at, updated_at, date, customer_id, number, bank_account, remarks
    )
    SELECT
        q.id,
        q.created_at,
        q.updated_at,
        q.date,
        cust.id,
        q.invoice_number,
        q.bank_account,
        COALESCE(q.remarks, '')
    FROM kds_quotations q
    JOIN customers cust
      ON cust.is_individual = false
     AND cust.company_id = q.customer_id
    WHERE NOT EXISTS (SELECT 1 FROM quotations existing WHERE existing.id = q.id);

    PERFORM setval(
        pg_get_serial_sequence('quotations', 'id'),
        GREATEST(COALESCE((SELECT MAX(id) FROM quotations), 1), 1),
        (SELECT MAX(id) FROM quotations) IS NOT NULL
    );

    FOR rec IN SELECT * FROM kds_quotation_material_lines ORDER BY id LOOP
        INSERT INTO quotation_goods_lines (
            created_at, updated_at, quotation_id, product_id, rate, quantity,
            variable_values, pre_tax_amount, remarks
        ) VALUES (
            rec.created_at, rec.updated_at, rec.invoice_id, rec.product_id, rec.final_cost, 1,
            CASE
                WHEN rec.variables IS NULL OR btrim(rec.variables::text) IN ('', 'null') THEN '{}'
                ELSE rec.variables::text
            END,
            rec.final_cost, NULLIF(BTRIM(COALESCE(rec.remarks, '')), '')
        )
        RETURNING id INTO new_id;

        INSERT INTO quotation_goods_line_taxes (line_id, tax_id)
        SELECT new_id, tax_id
        FROM kds_quotation_material_line_taxes
        WHERE line_id = rec.id;
    END LOOP;

    FOR rec IN SELECT * FROM kds_quotation_machine_lines ORDER BY id LOOP
        INSERT INTO quotation_service_lines (
            created_at, updated_at, quotation_id, service_id, rate, quantity,
            variable_values, pre_tax_amount, remarks
        ) VALUES (
            rec.created_at, rec.updated_at, rec.invoice_id, rec.product_id, rec.final_cost, 1,
            CASE
                WHEN rec.variables IS NULL OR btrim(rec.variables::text) IN ('', 'null') THEN '{}'
                ELSE rec.variables::text
            END,
            rec.final_cost, NULLIF(BTRIM(COALESCE(rec.remarks, '')), '')
        )
        RETURNING id INTO new_id;

        INSERT INTO quotation_service_line_taxes (line_id, tax_id)
        SELECT new_id, tax_id
        FROM kds_quotation_machine_line_taxes
        WHERE line_id = rec.id;
    END LOOP;

    PERFORM setval(
        pg_get_serial_sequence('quotation_goods_lines', 'id'),
        GREATEST(COALESCE((SELECT MAX(id) FROM quotation_goods_lines), 1), 1),
        (SELECT MAX(id) FROM quotation_goods_lines) IS NOT NULL
    );
    PERFORM setval(
        pg_get_serial_sequence('quotation_service_lines', 'id'),
        GREATEST(COALESCE((SELECT MAX(id) FROM quotation_service_lines), 1), 1),
        (SELECT MAX(id) FROM quotation_service_lines) IS NOT NULL
    );

    UPDATE quotation_preferences AS prefs
    SET
        quotation_number_format = COALESCE(NULLIF(BTRIM(src.quotation_number_format), ''), prefs.quotation_number_format),
        quotation_pdf_template = src.quotation_pdf_template,
        quotation_email_subject = src.quotation_email_subject,
        quotation_email_body = src.quotation_email_body,
        company_name = src.company_name,
        company_address = src.company_address,
        company_phone = src.company_phone,
        company_gstin = src.company_gstin,
        place_of_supply = src.place_of_supply,
        default_bank_account = src.default_bank_account,
        company_logo_vnode_id = src.company_logo_vnode_id,
        company_signature_vnode_id = src.company_signature_vnode_id,
        updated_at = NOW()
    FROM work_orders_preferences src
    WHERE prefs.id = 1 AND src.id = 1;

    FOR con IN
        SELECT child.relname AS table_name, constraint_row.conname AS constraint_name
        FROM pg_constraint constraint_row
        JOIN pg_class child ON child.oid = constraint_row.conrelid
        JOIN pg_class parent ON parent.oid = constraint_row.confrelid
        WHERE parent.relname = 'kds_quotations'
          AND constraint_row.contype = 'f'
    LOOP
        EXECUTE format(
            'ALTER TABLE %I DROP CONSTRAINT %I',
            con.table_name,
            con.constraint_name
        );
    END LOOP;

    IF to_regclass('public.draft_work_orders') IS NOT NULL THEN
        ALTER TABLE draft_work_orders
            ADD CONSTRAINT draft_work_orders_quotation_id_fkey
            FOREIGN KEY (quotation_id) REFERENCES quotations(id) ON DELETE SET NULL;
    END IF;
    IF to_regclass('public.kds_work_orders') IS NOT NULL THEN
        ALTER TABLE kds_work_orders
            ADD CONSTRAINT kds_work_orders_quotation_id_fkey
            FOREIGN KEY (quotation_id) REFERENCES quotations(id) ON DELETE SET NULL;
    END IF;

    ALTER TABLE work_orders_preferences
        DROP COLUMN IF EXISTS quotation_pdf_template,
        DROP COLUMN IF EXISTS quotation_number_format,
        DROP COLUMN IF EXISTS quotation_email_subject,
        DROP COLUMN IF EXISTS quotation_email_body,
        DROP COLUMN IF EXISTS default_bank_account;

    DROP TABLE IF EXISTS kds_quotation_material_line_taxes;
    DROP TABLE IF EXISTS kds_quotation_machine_line_taxes;
    DROP TABLE IF EXISTS kds_quotation_material_lines;
    DROP TABLE IF EXISTS kds_quotation_machine_lines;
    DROP TABLE IF EXISTS kds_quotations;
END $$;
"#;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        let backend = db.get_database_backend();
        if backend != DbBackend::Postgres {
            return Ok(());
        }
        db.execute_raw(Statement::from_string(backend, UP.to_string()))
            .await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
