//! One services product per machine.
//!
//! Machine lines were given their own products. Those lines now point at the
//! machine's product, and the per-line products are removed when nothing else
//! references them.

use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

async fn exec(db: &impl ConnectionTrait, backend: DbBackend, sql: &str) -> Result<(), DbErr> {
    db.execute_raw(Statement::from_string(backend, sql.to_string()))
        .await?;
    Ok(())
}

async fn table_exists(
    db: &impl ConnectionTrait,
    backend: DbBackend,
    table: &str,
) -> Result<bool, DbErr> {
    if backend == DbBackend::Postgres {
        let sql = format!(
            "SELECT COUNT(*) AS n FROM information_schema.tables WHERE table_schema = 'public' AND table_name = '{table}'"
        );
        let row = db
            .query_one_raw(Statement::from_string(backend, sql))
            .await?;
        let n: i64 = row.map(|r| r.try_get("", "n").unwrap_or(0)).unwrap_or(0);
        Ok(n > 0)
    } else {
        let sql =
            format!("SELECT 1 AS n FROM sqlite_master WHERE type = 'table' AND name = '{table}'");
        let row = db
            .query_one_raw(Statement::from_string(backend, sql))
            .await?;
        Ok(row.is_some())
    }
}

async fn column_exists(
    db: &impl ConnectionTrait,
    backend: DbBackend,
    table: &str,
    column: &str,
) -> Result<bool, DbErr> {
    if backend == DbBackend::Postgres {
        let sql = format!(
            "SELECT COUNT(*) AS n FROM information_schema.columns WHERE table_schema = 'public' AND table_name = '{table}' AND column_name = '{column}'"
        );
        let row = db
            .query_one_raw(Statement::from_string(backend, sql))
            .await?;
        let n: i64 = row.map(|r| r.try_get("", "n").unwrap_or(0)).unwrap_or(0);
        Ok(n > 0)
    } else {
        let sql =
            format!("SELECT 1 AS n FROM pragma_table_info('{table}') WHERE name = '{column}'");
        let row = db
            .query_one_raw(Statement::from_string(backend, sql))
            .await?;
        Ok(row.is_some())
    }
}

const LINE_TABLES: [&str; 3] = [
    "kds_quotation_machine_lines",
    "draft_work_order_machine_lines",
    "kds_work_order_machine_lines",
];

const LINE_PRODUCT_PREFIXES: [&str; 3] = ["kds-qmach-", "kds-dmach-", "kds-wmach-"];

fn line_product_map_sql() -> String {
    let mut parts = Vec::new();
    for table in LINE_TABLES {
        parts.push(format!(
            "SELECT l.product_id AS old_product_id, m.product_id AS machine_product_id
             FROM {table} l
             JOIN machinery_machines m ON m.id = l.machine_id
             WHERE l.machine_id IS NOT NULL
               AND m.product_id IS NOT NULL
               AND l.product_id <> m.product_id"
        ));
    }
    parts.join(" UNION ")
}

async fn repoint_line_products(
    db: &impl ConnectionTrait,
    backend: DbBackend,
    table: &str,
) -> Result<(), DbErr> {
    let map = line_product_map_sql();
    exec(
        db,
        backend,
        &format!(
            "UPDATE {table}
             SET product_id = (
                 SELECT machine_product_id FROM ({map}) mapped
                 WHERE mapped.old_product_id = {table}.product_id
             )
             WHERE EXISTS (
                 SELECT 1 FROM ({map}) mapped
                 WHERE mapped.old_product_id = {table}.product_id
             )"
        ),
    )
    .await
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        let backend = db.get_database_backend();
        if !table_exists(db, backend, "machinery_machines").await? {
            return Ok(());
        }
        if !column_exists(db, backend, "machinery_machines", "product_id").await? {
            let sql = if backend == DbBackend::Postgres {
                "ALTER TABLE machinery_machines ADD COLUMN product_id bigint"
            } else {
                "ALTER TABLE machinery_machines ADD COLUMN product_id INTEGER"
            };
            exec(db, backend, sql).await?;
        }
        if !table_exists(db, backend, "products").await? {
            return Ok(());
        }

        let services = if backend == DbBackend::Postgres {
            "'Services'::product_type"
        } else {
            "'Services'"
        };
        let id_text = if backend == DbBackend::Postgres {
            "m.id::text"
        } else {
            "m.id"
        };
        let reference = format!("'kds-machine-' || {id_text}");
        let machine_vars = if backend == DbBackend::Postgres {
            "COALESCE(m.variables::text, '{}')"
        } else {
            "COALESCE(m.variables, '{}')"
        };
        let trim = if backend == DbBackend::Postgres {
            "btrim"
        } else {
            "trim"
        };
        let formula = format!(
            "CASE WHEN {trim}(COALESCE(m.cost_formula, '')) = '' THEN '0' ELSE m.cost_formula END"
        );
        let name_expr = format!(
            "CASE WHEN {trim}(COALESCE(m.name, '')) = '' THEN 'Machine ' || {id_text} ELSE m.name END"
        );

        exec(
            db,
            backend,
            &format!(
                "INSERT INTO products (
                    created_at, updated_at, product_type, reference, name, hsn_code,
                    variables, base_price_formula, sales_price_formula
                )
                SELECT
                    CURRENT_TIMESTAMP,
                    CURRENT_TIMESTAMP,
                    {services},
                    {reference},
                    {name_expr},
                    0,
                    {machine_vars},
                    {formula},
                    {formula}
                FROM machinery_machines m
                WHERE m.product_id IS NULL
                  AND NOT EXISTS (
                      SELECT 1 FROM products p WHERE p.reference = {reference}
                  )"
            ),
        )
        .await?;

        let reference_match = if backend == DbBackend::Postgres {
            "'kds-machine-' || machinery_machines.id::text"
        } else {
            "'kds-machine-' || machinery_machines.id"
        };
        exec(
            db,
            backend,
            &format!(
                "UPDATE machinery_machines
                 SET product_id = (
                     SELECT p.id FROM products p
                     WHERE p.reference = {reference_match}
                 )
                 WHERE product_id IS NULL"
            ),
        )
        .await?;

        let row = db
            .query_one_raw(Statement::from_string(
                backend,
                "SELECT COUNT(*) AS n FROM machinery_machines WHERE product_id IS NULL".to_string(),
            ))
            .await?;
        let missing: i64 = row.map(|r| r.try_get("", "n").unwrap_or(0)).unwrap_or(0);
        if missing > 0 {
            return Err(DbErr::Custom(format!(
                "machinery_machines has {missing} rows without a product"
            )));
        }

        // Point anything still using a per-line product at that machine's product
        // before the lines themselves move.
        for table in [
            "kds_quotation_material_lines",
            "draft_work_order_material_lines",
            "kds_work_order_material_lines",
            "work_order_components",
            "draft_invoice_lines",
            "posted_invoice_lines",
            "cancelled_invoice_lines",
            "draft_purchase_lines",
            "posted_purchase_lines",
            "cancelled_purchase_lines",
        ] {
            if table_exists(db, backend, table).await? {
                repoint_line_products(db, backend, table).await?;
            }
        }

        for table in LINE_TABLES {
            if !table_exists(db, backend, table).await? {
                continue;
            }
            exec(
                db,
                backend,
                &format!(
                    "UPDATE {table}
                     SET product_id = (
                         SELECT m.product_id FROM machinery_machines m
                         WHERE m.id = {table}.machine_id
                     )
                     WHERE machine_id IS NOT NULL
                       AND EXISTS (
                           SELECT 1 FROM machinery_machines m
                           WHERE m.id = {table}.machine_id
                             AND m.product_id IS NOT NULL
                       )"
                ),
            )
            .await?;
        }

        let mut prefix_sql = String::new();
        for (i, prefix) in LINE_PRODUCT_PREFIXES.iter().enumerate() {
            if i > 0 {
                prefix_sql.push_str(" OR ");
            }
            prefix_sql.push_str(&format!("p.reference LIKE '{prefix}%'"));
        }
        let mut guards = String::new();
        for table in [
            "kds_quotation_machine_lines",
            "draft_work_order_machine_lines",
            "kds_work_order_machine_lines",
            "kds_quotation_material_lines",
            "draft_work_order_material_lines",
            "kds_work_order_material_lines",
            "work_order_components",
            "draft_invoice_lines",
            "posted_invoice_lines",
            "cancelled_invoice_lines",
            "draft_purchase_lines",
            "posted_purchase_lines",
            "cancelled_purchase_lines",
        ] {
            if table_exists(db, backend, table).await? {
                guards.push_str(&format!(
                    " AND NOT EXISTS (SELECT 1 FROM {table} t WHERE t.product_id = p.id)"
                ));
            }
        }
        exec(
            db,
            backend,
            &format!("DELETE FROM products p WHERE ({prefix_sql}){guards}"),
        )
        .await?;

        if backend == DbBackend::Postgres {
            exec(
                db,
                backend,
                "ALTER TABLE machinery_machines ALTER COLUMN product_id SET NOT NULL",
            )
            .await?;
            exec(
                db,
                backend,
                "ALTER TABLE machinery_machines DROP CONSTRAINT IF EXISTS fk_machinery_machines_product_id",
            )
            .await?;
            exec(
                db,
                backend,
                "ALTER TABLE machinery_machines ADD CONSTRAINT fk_machinery_machines_product_id FOREIGN KEY (product_id) REFERENCES products(id) ON DELETE RESTRICT",
            )
            .await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        let backend = db.get_database_backend();
        if backend == DbBackend::Postgres && table_exists(db, backend, "machinery_machines").await?
        {
            let _ = exec(
                db,
                backend,
                "ALTER TABLE machinery_machines DROP CONSTRAINT IF EXISTS fk_machinery_machines_product_id",
            )
            .await;
            let _ = exec(
                db,
                backend,
                "ALTER TABLE machinery_machines DROP COLUMN IF EXISTS product_id",
            )
            .await;
        }
        Ok(())
    }
}
