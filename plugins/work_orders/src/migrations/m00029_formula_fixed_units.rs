//! Quantity unit `Number` and decimal unit `Seconds` on component schemas and line measures.

use sea_orm_migration::prelude::*;

use lariv_plugin_finance_products::migrations::formula_measure_sql::REFRESH_FORMULA_FIXED_UNITS;

#[derive(DeriveMigrationName)]
pub struct Migration;

const REFRESH: &str = r#"
UPDATE work_order_components
SET variables = lariv_refresh_formula_schema_units(variables::jsonb)
WHERE variables IS NOT NULL AND variables::text NOT IN ('{}', 'null');

UPDATE draft_work_order_lines
SET variables = lariv_refresh_formula_value_units(variables::jsonb)
WHERE variables IS NOT NULL AND variables::text NOT IN ('{}', 'null');

UPDATE work_order_lines
SET variables = lariv_refresh_formula_value_units(variables::jsonb)
WHERE variables IS NOT NULL AND variables::text NOT IN ('{}', 'null');

UPDATE draft_work_order_material_lines
SET variables = lariv_refresh_formula_value_units(variables::jsonb)
WHERE variables IS NOT NULL AND variables::text NOT IN ('{}', 'null');

UPDATE draft_work_proposal_lines
SET variables = lariv_refresh_formula_value_units(variables::jsonb)
WHERE variables IS NOT NULL AND variables::text NOT IN ('{}', 'null');

UPDATE kds_quotation_material_lines
SET variables = lariv_refresh_formula_value_units(variables::jsonb)
WHERE variables IS NOT NULL AND variables::text NOT IN ('{}', 'null');

UPDATE kds_work_order_material_lines
SET variables = lariv_refresh_formula_value_units(variables::jsonb)
WHERE variables IS NOT NULL AND variables::text NOT IN ('{}', 'null');

UPDATE draft_work_order_machine_lines
SET variables = lariv_refresh_formula_value_units(variables::jsonb)
WHERE variables IS NOT NULL AND variables::text NOT IN ('{}', 'null');

UPDATE kds_quotation_machine_lines
SET variables = lariv_refresh_formula_value_units(variables::jsonb)
WHERE variables IS NOT NULL AND variables::text NOT IN ('{}', 'null');

UPDATE kds_work_order_machine_lines
SET variables = lariv_refresh_formula_value_units(variables::jsonb)
WHERE variables IS NOT NULL AND variables::text NOT IN ('{}', 'null');
"#;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        conn.execute_unprepared(REFRESH_FORMULA_FIXED_UNITS).await?;
        conn.execute_unprepared(REFRESH).await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
