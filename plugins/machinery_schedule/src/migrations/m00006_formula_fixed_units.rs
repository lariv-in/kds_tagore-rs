//! Quantity unit `Number` and decimal unit `Seconds` on machine schemas.

use sea_orm_migration::prelude::*;

use lariv_plugin_finance_products::migrations::formula_measure_sql::REFRESH_FORMULA_FIXED_UNITS;

#[derive(DeriveMigrationName)]
pub struct Migration;

const REFRESH_MACHINES: &str = r#"
UPDATE machinery_machines
SET variables = lariv_refresh_formula_schema_units(variables::jsonb)
WHERE variables IS NOT NULL AND variables::text NOT IN ('{}', 'null');
"#;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        conn.execute_unprepared(REFRESH_FORMULA_FIXED_UNITS).await?;
        conn.execute_unprepared(REFRESH_MACHINES).await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
