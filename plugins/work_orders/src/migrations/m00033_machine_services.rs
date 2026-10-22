//! Point machine links at `services` instead of `products`.
//!
//! Finance services copies service-typed products and keeps the old product id
//! in `services.legacy_product_id`. Machine rows and machine lines still hold
//! that product id. This copies any machine product that was not copied yet,
//! rewrites those ids to `services.id`, and retargets the foreign keys.

use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const UP: &str = r#"
DO $$
DECLARE
    tbl text;
    missing boolean;
    tables text[] := ARRAY[
        'machinery_machines',
        'kds_quotation_machine_lines',
        'draft_work_order_machine_lines',
        'kds_work_order_machine_lines'
    ];
BEGIN
    IF to_regclass('public.services') IS NULL THEN
        RAISE EXCEPTION 'services table is missing; install finance services before work orders';
    END IF;

    CREATE TEMP TABLE kds_machine_product_ids (
        product_id bigint PRIMARY KEY
    ) ON COMMIT DROP;

    FOREACH tbl IN ARRAY tables LOOP
        IF to_regclass('public.' || tbl) IS NULL THEN
            CONTINUE;
        END IF;
        IF NOT EXISTS (
            SELECT 1
            FROM information_schema.columns
            WHERE table_schema = 'public'
              AND table_name = tbl
              AND column_name = 'product_id'
        ) THEN
            CONTINUE;
        END IF;
        EXECUTE format(
            'ALTER TABLE %I DROP CONSTRAINT IF EXISTS %I',
            tbl,
            'fk_' || tbl || '_product_id'
        );
        EXECUTE format(
            'INSERT INTO kds_machine_product_ids (product_id)
             SELECT DISTINCT product_id FROM %I
             WHERE product_id IS NOT NULL AND product_id > 0
             ON CONFLICT DO NOTHING',
            tbl
        );
    END LOOP;

    IF to_regclass('public.products') IS NOT NULL
       AND EXISTS (
           SELECT 1
           FROM information_schema.columns
           WHERE table_schema = 'public'
             AND table_name = 'services'
             AND column_name = 'legacy_product_id'
       )
    THEN
        INSERT INTO services (
            created_at, updated_at, legacy_product_id, reference, remarks, name,
            sac_code, variables, base_price_formula, sales_price_formula
        )
        SELECT
            p.created_at,
            p.updated_at,
            p.id,
            CASE
                WHEN p.reference IS NOT NULL AND EXISTS (
                    SELECT 1 FROM services existing WHERE existing.reference = p.reference
                ) THEN NULL
                ELSE p.reference
            END,
            p.remarks,
            p.name,
            COALESCE(p.hsn_code, 0),
            COALESCE(p.variables, '{}'),
            COALESCE(NULLIF(btrim(p.base_price_formula), ''), '0'),
            COALESCE(
                NULLIF(btrim(p.sales_price_formula), ''),
                NULLIF(btrim(p.base_price_formula), ''),
                '0'
            )
        FROM products p
        JOIN kds_machine_product_ids ids ON ids.product_id = p.id
        WHERE NOT EXISTS (
            SELECT 1 FROM services s WHERE s.legacy_product_id = p.id
        );

        FOREACH tbl IN ARRAY tables LOOP
            IF to_regclass('public.' || tbl) IS NULL THEN
                CONTINUE;
            END IF;
            IF NOT EXISTS (
                SELECT 1
                FROM information_schema.columns
                WHERE table_schema = 'public'
                  AND table_name = tbl
                  AND column_name = 'product_id'
            ) THEN
                CONTINUE;
            END IF;
            EXECUTE format(
                'UPDATE %I AS line
                 SET product_id = service.id
                 FROM services AS service
                 WHERE service.legacy_product_id = line.product_id',
                tbl
            );
        END LOOP;
    END IF;

    FOREACH tbl IN ARRAY tables LOOP
        IF to_regclass('public.' || tbl) IS NULL THEN
            CONTINUE;
        END IF;
        IF NOT EXISTS (
            SELECT 1
            FROM information_schema.columns
            WHERE table_schema = 'public'
              AND table_name = tbl
              AND column_name = 'product_id'
        ) THEN
            CONTINUE;
        END IF;
        EXECUTE format(
            'SELECT EXISTS (
                SELECT 1 FROM %I AS line
                WHERE line.product_id IS NOT NULL
                  AND NOT EXISTS (
                      SELECT 1 FROM services AS service WHERE service.id = line.product_id
                  )
            )',
            tbl
        ) INTO missing;
        IF missing THEN
            RAISE EXCEPTION 'machine links in % still do not match a service', tbl;
        END IF;
        EXECUTE format(
            'ALTER TABLE %I ADD CONSTRAINT %I FOREIGN KEY (product_id) REFERENCES services(id) ON UPDATE CASCADE ON DELETE RESTRICT',
            tbl,
            'fk_' || tbl || '_product_id'
        );
    END LOOP;
END $$;
"#;

const DOWN: &str = r#"
DO $$
DECLARE
    tbl text;
    tables text[] := ARRAY[
        'machinery_machines',
        'kds_quotation_machine_lines',
        'draft_work_order_machine_lines',
        'kds_work_order_machine_lines'
    ];
BEGIN
    FOREACH tbl IN ARRAY tables LOOP
        IF to_regclass('public.' || tbl) IS NULL THEN
            CONTINUE;
        END IF;
        EXECUTE format(
            'ALTER TABLE %I DROP CONSTRAINT IF EXISTS %I',
            tbl,
            'fk_' || tbl || '_product_id'
        );
    END LOOP;
END $$;
"#;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        if db.get_database_backend() != DbBackend::Postgres {
            return Ok(());
        }
        db.execute_raw(Statement::from_string(DbBackend::Postgres, UP.to_string()))
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        if db.get_database_backend() != DbBackend::Postgres {
            return Ok(());
        }
        db.execute_raw(Statement::from_string(
            DbBackend::Postgres,
            DOWN.to_string(),
        ))
        .await?;
        Ok(())
    }
}
