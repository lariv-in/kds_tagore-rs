//! Drop the per-line products left behind by the quotation product migrations.
//!
//! Machine lines used to each insert their own services product (`kds-{uuid}`,
//! and the earlier `kds-qmach-` / `kds-dmach-` / `kds-wmach-` references). Lines
//! now share the machine's product. This repoints anything still holding one of
//! those per-line products, then deletes the ones nothing references.
//! Catalog products and the component / machine products stay.

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

const LINE_TABLES: [&str; 3] = [
    "kds_quotation_machine_lines",
    "draft_work_order_machine_lines",
    "kds_work_order_machine_lines",
];

/// Every table with a foreign key to `products(id)`.
const PRODUCT_TABLES: [&str; 15] = [
    "kds_quotation_machine_lines",
    "draft_work_order_machine_lines",
    "kds_work_order_machine_lines",
    "kds_quotation_material_lines",
    "draft_work_order_material_lines",
    "kds_work_order_material_lines",
    "work_order_components",
    "machinery_machines",
    "draft_invoice_lines",
    "posted_invoice_lines",
    "cancelled_invoice_lines",
    "draft_purchase_lines",
    "posted_purchase_lines",
    "cancelled_purchase_lines",
    "product_taxes",
];

fn per_line_reference_sql(alias: &str) -> String {
    format!(
        "({alias}.reference LIKE 'kds-________-____-____-____-____________' \
         OR {alias}.reference LIKE 'kds-qmach-%' \
         OR {alias}.reference LIKE 'kds-dmach-%' \
         OR {alias}.reference LIKE 'kds-wmach-%')"
    )
}

fn line_product_map_sql(line_tables: &[&str]) -> String {
    let arms = line_tables
        .iter()
        .map(|table| {
            format!("SELECT product_id, machine_id FROM {table} WHERE machine_id IS NOT NULL")
        })
        .collect::<Vec<_>>()
        .join(" UNION ALL ");
    let predicate = per_line_reference_sql("p");
    format!(
        "SELECT l.product_id AS old_product_id, MIN(m.product_id) AS machine_product_id
         FROM ({arms}) l
         JOIN machinery_machines m ON m.id = l.machine_id
         JOIN products p ON p.id = l.product_id
         WHERE m.product_id IS NOT NULL
           AND l.product_id <> m.product_id
           AND {predicate}
         GROUP BY l.product_id"
    )
}

async fn repoint(
    db: &impl ConnectionTrait,
    backend: DbBackend,
    table: &str,
    map: &str,
) -> Result<(), DbErr> {
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
        if !table_exists(db, backend, "products").await? {
            return Ok(());
        }

        let mut line_tables = Vec::new();
        for table in LINE_TABLES {
            if table_exists(db, backend, table).await? {
                line_tables.push(table);
            }
        }
        if !line_tables.is_empty() && table_exists(db, backend, "machinery_machines").await? {
            let map = line_product_map_sql(&line_tables);
            for table in PRODUCT_TABLES {
                if table == "product_taxes" {
                    continue;
                }
                if table_exists(db, backend, table).await? {
                    repoint(db, backend, table, &map).await?;
                }
            }
        }

        let predicate = per_line_reference_sql("p");
        let mut guards = String::new();
        for table in PRODUCT_TABLES {
            if table_exists(db, backend, table).await? {
                guards.push_str(&format!(
                    " AND NOT EXISTS (SELECT 1 FROM {table} t WHERE t.product_id = p.id)"
                ));
            }
        }
        exec(
            db,
            backend,
            &format!(
                "DELETE FROM products WHERE id IN (
                    SELECT p.id FROM products p WHERE {predicate}{guards}
                )"
            ),
        )
        .await?;

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
