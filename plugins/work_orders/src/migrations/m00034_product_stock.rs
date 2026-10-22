//! Give products that have no stock a quantity stock before the finance catalog
//! cleanup requires `products.stock_id`.

use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const UP: &str = r#"
DO $$
DECLARE
    stock bigint;
BEGIN
    IF to_regclass('public.products') IS NULL OR to_regclass('public.inventory_stocks') IS NULL THEN
        RETURN;
    END IF;
    IF NOT EXISTS (
        SELECT 1
        FROM information_schema.columns
        WHERE table_schema = 'public'
          AND table_name = 'products'
          AND column_name = 'stock_id'
    ) THEN
        RETURN;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM products WHERE stock_id IS NULL) THEN
        RETURN;
    END IF;

    SELECT id INTO stock
    FROM inventory_stocks
    WHERE name = 'Components' AND qty_type = 'quantity'
    ORDER BY id
    LIMIT 1;

    IF stock IS NULL THEN
        INSERT INTO inventory_stocks (created_at, updated_at, name, qty_type, qty_unit)
        VALUES (NOW(), NOW(), 'Components', 'quantity', 'Number')
        RETURNING id INTO stock;
    END IF;

    UPDATE products SET stock_id = stock WHERE stock_id IS NULL;
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

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
