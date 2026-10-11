//! Link components and machine lines to finance products.
//!
//! Each component gets one goods product. Each machine line gets one services
//! product. Variable schemas and cost formulas are copied onto those products.
//! Material lines point at their component's product.

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

async fn add_product_id(
    db: &impl ConnectionTrait,
    backend: DbBackend,
    table: &str,
) -> Result<(), DbErr> {
    if column_exists(db, backend, table, "product_id").await? {
        return Ok(());
    }
    let sql = if backend == DbBackend::Postgres {
        format!("ALTER TABLE {table} ADD COLUMN product_id bigint")
    } else {
        format!("ALTER TABLE {table} ADD COLUMN product_id INTEGER")
    };
    exec(db, backend, &sql).await
}

fn null_product_count_sql(table: &str) -> String {
    format!("SELECT COUNT(*) AS n FROM {table} WHERE product_id IS NULL")
}

async fn assert_linked(
    db: &impl ConnectionTrait,
    backend: DbBackend,
    table: &str,
) -> Result<(), DbErr> {
    let row = db
        .query_one_raw(Statement::from_string(
            backend,
            null_product_count_sql(table),
        ))
        .await?;
    let n: i64 = row.map(|r| r.try_get("", "n").unwrap_or(0)).unwrap_or(0);
    if n > 0 {
        return Err(DbErr::Custom(format!(
            "{table} has {n} rows without a product"
        )));
    }
    Ok(())
}

async fn backfill(db: &impl ConnectionTrait, backend: DbBackend) -> Result<(), DbErr> {
    let goods = if backend == DbBackend::Postgres {
        "'Goods'::product_type"
    } else {
        "'Goods'"
    };
    let services = if backend == DbBackend::Postgres {
        "'Services'::product_type"
    } else {
        "'Services'"
    };
    let component_vars = if backend == DbBackend::Postgres {
        "COALESCE(c.variables::text, '{}')"
    } else {
        "COALESCE(c.variables, '{}')"
    };
    let component_formula =
        "CASE WHEN trim(COALESCE(c.cost_formula, '')) = '' THEN '0' ELSE c.cost_formula END";
    let component_id_text = if backend == DbBackend::Postgres {
        "c.id::text"
    } else {
        "c.id"
    };
    let component_ref = format!("'kds-component-' || {component_id_text}");

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
                {goods},
                {component_ref},
                CASE WHEN trim(COALESCE(c.name, '')) = '' THEN 'Component ' || {component_id_text} ELSE c.name END,
                0,
                {component_vars},
                {component_formula},
                {component_formula}
            FROM work_order_components c
            WHERE NOT EXISTS (
                SELECT 1 FROM products p WHERE p.reference = {component_ref}
            )"
        ),
    )
    .await?;

    let component_ref_match = if backend == DbBackend::Postgres {
        "'kds-component-' || work_order_components.id::text"
    } else {
        "'kds-component-' || work_order_components.id"
    };
    exec(
        db,
        backend,
        &format!(
            "UPDATE work_order_components
             SET product_id = (
                 SELECT p.id FROM products p
                 WHERE p.reference = {component_ref_match}
             )
             WHERE product_id IS NULL"
        ),
    )
    .await?;

    for table in [
        "kds_quotation_material_lines",
        "draft_work_order_material_lines",
        "kds_work_order_material_lines",
    ] {
        if !table_exists(db, backend, table).await? {
            continue;
        }
        exec(
            db,
            backend,
            &format!(
                "UPDATE {table}
                 SET product_id = (
                     SELECT c.product_id FROM work_order_components c
                     WHERE c.id = {table}.component_id
                 )
                 WHERE product_id IS NULL"
            ),
        )
        .await?;
    }

    backfill_machine_lines(
        db,
        backend,
        services,
        "kds_quotation_machine_lines",
        "kds-qmach-",
        true,
    )
    .await?;
    backfill_machine_lines(
        db,
        backend,
        services,
        "draft_work_order_machine_lines",
        "kds-dmach-",
        false,
    )
    .await?;
    backfill_machine_lines(
        db,
        backend,
        services,
        "kds_work_order_machine_lines",
        "kds-wmach-",
        true,
    )
    .await?;

    for table in [
        "work_order_components",
        "kds_quotation_material_lines",
        "draft_work_order_material_lines",
        "kds_work_order_material_lines",
        "kds_quotation_machine_lines",
        "draft_work_order_machine_lines",
        "kds_work_order_machine_lines",
    ] {
        if table_exists(db, backend, table).await? {
            assert_linked(db, backend, table).await?;
        }
    }

    if backend == DbBackend::Postgres {
        for table in [
            "work_order_components",
            "kds_quotation_material_lines",
            "draft_work_order_material_lines",
            "kds_work_order_material_lines",
            "kds_quotation_machine_lines",
            "draft_work_order_machine_lines",
            "kds_work_order_machine_lines",
        ] {
            if !table_exists(db, backend, table).await? {
                continue;
            }
            exec(
                db,
                backend,
                &format!("ALTER TABLE {table} ALTER COLUMN product_id SET NOT NULL"),
            )
            .await?;
            let fk = format!("fk_{table}_product_id");
            exec(
                db,
                backend,
                &format!("ALTER TABLE {table} DROP CONSTRAINT IF EXISTS {fk}"),
            )
            .await?;
            exec(
                db,
                backend,
                &format!(
                    "ALTER TABLE {table} ADD CONSTRAINT {fk} FOREIGN KEY (product_id) REFERENCES products(id) ON DELETE RESTRICT"
                ),
            )
            .await?;
        }
    }

    Ok(())
}

async fn backfill_machine_lines(
    db: &impl ConnectionTrait,
    backend: DbBackend,
    services: &str,
    table: &str,
    reference_prefix: &str,
    has_name: bool,
) -> Result<(), DbErr> {
    if !table_exists(db, backend, table).await? {
        return Ok(());
    }
    let id_text = if backend == DbBackend::Postgres {
        "l.id::text"
    } else {
        "l.id"
    };
    let reference = format!("'{reference_prefix}' || {id_text}");
    let machine_vars = if backend == DbBackend::Postgres {
        "COALESCE(m.variables::text, '{}')"
    } else {
        "COALESCE(m.variables, '{}')"
    };
    let formula = if backend == DbBackend::Postgres {
        "CASE
            WHEN m.id IS NOT NULL AND btrim(COALESCE(m.cost_formula, '')) <> '' THEN m.cost_formula
            WHEN l.final_cost = 0 THEN '0'
            ELSE trim(trailing '.' FROM trim(trailing '0' FROM l.final_cost::text))
        END"
    } else {
        "CASE
            WHEN m.id IS NOT NULL AND trim(COALESCE(m.cost_formula, '')) <> '' THEN m.cost_formula
            WHEN l.final_cost IS NULL OR l.final_cost = 0 THEN '0'
            ELSE CAST(l.final_cost AS TEXT)
        END"
    };
    let name_expr = if has_name {
        format!(
            "CASE
                WHEN trim(COALESCE(l.name, '')) <> '' THEN l.name
                WHEN m.name IS NOT NULL AND trim(m.name) <> '' THEN m.name
                ELSE 'Machine line ' || {id_text}
            END"
        )
    } else {
        format!(
            "CASE
                WHEN m.name IS NOT NULL AND trim(m.name) <> '' THEN m.name
                ELSE 'Machine line ' || {id_text}
            END"
        )
    };

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
            FROM {table} l
            LEFT JOIN machinery_machines m ON m.id = l.machine_id
            WHERE NOT EXISTS (
                SELECT 1 FROM products p WHERE p.reference = {reference}
            )"
        ),
    )
    .await?;

    let reference_match = if backend == DbBackend::Postgres {
        format!("'{reference_prefix}' || {table}.id::text")
    } else {
        format!("'{reference_prefix}' || {table}.id")
    };
    exec(
        db,
        backend,
        &format!(
            "UPDATE {table}
             SET product_id = (
                 SELECT p.id FROM products p WHERE p.reference = {reference_match}
             )
             WHERE product_id IS NULL"
        ),
    )
    .await
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        let backend = db.get_database_backend();

        for table in [
            "work_order_components",
            "kds_quotation_material_lines",
            "draft_work_order_material_lines",
            "kds_work_order_material_lines",
            "kds_quotation_machine_lines",
            "draft_work_order_machine_lines",
            "kds_work_order_machine_lines",
        ] {
            if table_exists(db, backend, table).await? {
                add_product_id(db, backend, table).await?;
            }
        }

        if table_exists(db, backend, "products").await? {
            backfill(db, backend).await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        let backend = db.get_database_backend();

        if backend == DbBackend::Postgres {
            for table in [
                "kds_work_order_machine_lines",
                "draft_work_order_machine_lines",
                "kds_quotation_machine_lines",
                "kds_work_order_material_lines",
                "draft_work_order_material_lines",
                "kds_quotation_material_lines",
                "work_order_components",
            ] {
                if table_exists(db, backend, table).await? {
                    let fk = format!("fk_{table}_product_id");
                    let _ = exec(
                        db,
                        backend,
                        &format!("ALTER TABLE {table} DROP CONSTRAINT IF EXISTS {fk}"),
                    )
                    .await;
                    let _ = exec(
                        db,
                        backend,
                        &format!("ALTER TABLE {table} DROP COLUMN IF EXISTS product_id"),
                    )
                    .await;
                }
            }
        }

        Ok(())
    }
}
