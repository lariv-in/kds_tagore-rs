//! Accepted units on machine variable definitions.

use sea_orm_migration::prelude::*;

use lariv_plugin_finance_products::migrations::formula_measure_sql::ENSURE_FORMULA_MEASURE;

#[derive(DeriveMigrationName)]
pub struct Migration;

const MIGRATE_MACHINES: &str = r#"
UPDATE machinery_machines
SET variables = lariv_migrate_formula_schema(variables::jsonb)
WHERE variables IS NOT NULL AND variables::text NOT IN ('{}', 'null');
"#;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        conn.execute_unprepared(ENSURE_FORMULA_MEASURE).await?;
        conn.execute_unprepared(MIGRATE_MACHINES).await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
