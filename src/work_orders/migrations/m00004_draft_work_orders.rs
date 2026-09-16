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
            db.execute(Statement::from_string(
                backend,
                r#"
                DO $$
                BEGIN
                    IF EXISTS (SELECT FROM pg_tables WHERE schemaname = 'public' AND tablename = 'work_orders')
                       AND NOT EXISTS (SELECT FROM pg_tables WHERE schemaname = 'public' AND tablename = 'draft_work_orders') THEN
                        ALTER TABLE work_orders RENAME TO draft_work_orders;
                    END IF;
                END $$;
                "#.to_string(),
            ))
            .await?;

            db.execute(Statement::from_string(
                backend,
                r#"
                CREATE TABLE IF NOT EXISTS draft_work_orders (
                    id bigserial PRIMARY KEY,
                    created_at timestamp with time zone,
                    updated_at timestamp with time zone,
                    order_number text NOT NULL,
                    customer_id bigint NOT NULL
                );
                "#.to_string(),
            ))
            .await?;

            db.execute(Statement::from_string(
                backend,
                "ALTER TABLE draft_work_orders DROP COLUMN IF EXISTS component_id;".to_string(),
            ))
            .await?;

            db.execute(Statement::from_string(
                backend,
                "ALTER TABLE draft_work_orders DROP COLUMN IF EXISTS quantity;".to_string(),
            ))
            .await?;

            db.execute(Statement::from_string(
                backend,
                "ALTER TABLE draft_work_orders DROP COLUMN IF EXISTS variables;".to_string(),
            ))
            .await?;

            db.execute(Statement::from_string(
                backend,
                r#"
                CREATE TABLE IF NOT EXISTS draft_work_order_lines (
                    id bigserial PRIMARY KEY,
                    created_at timestamp with time zone,
                    updated_at timestamp with time zone,
                    draft_work_order_id bigint NOT NULL REFERENCES draft_work_orders(id) ON DELETE CASCADE,
                    component_id bigint NOT NULL REFERENCES work_order_components(id) ON DELETE RESTRICT,
                    variables json NOT NULL DEFAULT '{}',
                    quantity numeric(16,4) NOT NULL DEFAULT 1,
                    unit_weight numeric(16,4) NOT NULL DEFAULT 0,
                    material_rate numeric(16,4) NOT NULL DEFAULT 0,
                    final_cost numeric(16,4) NOT NULL DEFAULT 0,
                    extra_data json NOT NULL DEFAULT '{}'
                );
                "#.to_string(),
            ))
            .await?;

            db.execute(Statement::from_string(
                backend,
                "CREATE OR REPLACE VIEW draft_work_proposals AS SELECT * FROM draft_work_orders;".to_string(),
            ))
            .await?;

            db.execute(Statement::from_string(
                backend,
                "CREATE OR REPLACE VIEW draft_work_proposal_lines AS SELECT * FROM draft_work_order_lines;".to_string(),
            ))
            .await?;
        } else {
            // SQLite or other backends
            db.execute(Statement::from_string(
                backend,
                r#"
                CREATE TABLE IF NOT EXISTS draft_work_orders (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    created_at TEXT,
                    updated_at TEXT,
                    order_number TEXT NOT NULL,
                    customer_id INTEGER NOT NULL
                );
                "#.to_string(),
            ))
            .await?;

            db.execute(Statement::from_string(
                backend,
                r#"
                CREATE TABLE IF NOT EXISTS draft_work_order_lines (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    created_at TEXT,
                    updated_at TEXT,
                    draft_work_order_id INTEGER NOT NULL REFERENCES draft_work_orders(id) ON DELETE CASCADE,
                    component_id INTEGER NOT NULL REFERENCES work_order_components(id) ON DELETE RESTRICT,
                    variables TEXT NOT NULL DEFAULT '{}',
                    quantity NUMERIC NOT NULL DEFAULT 1,
                    unit_weight NUMERIC NOT NULL DEFAULT 0,
                    material_rate NUMERIC NOT NULL DEFAULT 0,
                    final_cost NUMERIC NOT NULL DEFAULT 0,
                    extra_data TEXT NOT NULL DEFAULT '{}'
                );
                "#.to_string(),
            ))
            .await?;

            db.execute(Statement::from_string(
                backend,
                r#"
                CREATE VIEW IF NOT EXISTS draft_work_proposals AS SELECT * FROM draft_work_orders;
                "#.to_string(),
            ))
            .await?;
            db.execute(Statement::from_string(
                backend,
                r#"
                CREATE VIEW IF NOT EXISTS draft_work_proposal_lines AS SELECT * FROM draft_work_order_lines;
                "#.to_string(),
            ))
            .await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        let backend = db.get_database_backend();
        db.execute(Statement::from_string(
            backend,
            "DROP VIEW IF EXISTS draft_work_proposal_lines;".to_string(),
        ))
        .await?;
        db.execute(Statement::from_string(
            backend,
            "DROP VIEW IF EXISTS draft_work_proposals;".to_string(),
        ))
        .await?;
        db.execute(Statement::from_string(
            backend,
            "DROP TABLE IF EXISTS draft_work_order_lines;".to_string(),
        ))
        .await?;
        db.execute(Statement::from_string(
            backend,
            "DROP TABLE IF EXISTS draft_work_orders;".to_string(),
        ))
        .await?;
        Ok(())
    }
}
