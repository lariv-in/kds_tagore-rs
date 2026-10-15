//! Rewrite component schemas and work-order line values into formula measures.

use sea_orm_migration::prelude::*;

use lariv_plugin_finance_products::migrations::formula_measure_sql::ENSURE_FORMULA_MEASURE;

#[derive(DeriveMigrationName)]
pub struct Migration;

const MIGRATE: &str = r#"
UPDATE work_order_components
SET variables = lariv_migrate_formula_schema(variables::jsonb)
WHERE variables IS NOT NULL AND variables::text NOT IN ('{}', 'null');

UPDATE draft_work_order_lines AS line
SET variables = lariv_migrate_formula_values(
    line.variables::jsonb, component.variables::jsonb, COALESCE(line.extra_data::jsonb, '{}'::jsonb)
)
FROM work_order_components AS component
WHERE component.id = line.component_id
  AND line.variables IS NOT NULL
  AND line.variables::text NOT IN ('{}', 'null');

UPDATE work_order_lines AS line
SET variables = lariv_migrate_formula_values(
    line.variables::jsonb, component.variables::jsonb, COALESCE(line.extra_data::jsonb, '{}'::jsonb)
)
FROM work_order_components AS component
WHERE component.id = line.component_id
  AND line.variables IS NOT NULL
  AND line.variables::text NOT IN ('{}', 'null');

UPDATE draft_work_order_material_lines AS line
SET variables = lariv_migrate_formula_values(
    line.variables::jsonb, component.variables::jsonb, COALESCE(line.extra_data::jsonb, '{}'::jsonb)
)
FROM work_order_components AS component
WHERE component.id = line.component_id
  AND line.variables IS NOT NULL
  AND line.variables::text NOT IN ('{}', 'null');

UPDATE draft_work_proposal_lines AS line
SET variables = lariv_migrate_formula_values(
    line.variables::jsonb, component.variables::jsonb, COALESCE(line.extra_data::jsonb, '{}'::jsonb)
)
FROM work_order_components AS component
WHERE component.id = line.component_id
  AND line.variables IS NOT NULL
  AND line.variables::text NOT IN ('{}', 'null');

UPDATE kds_quotation_material_lines AS line
SET variables = lariv_migrate_formula_values(
    line.variables::jsonb, component.variables::jsonb, COALESCE(line.extra_data::jsonb, '{}'::jsonb)
)
FROM work_order_components AS component
WHERE component.id = line.component_id
  AND line.variables IS NOT NULL
  AND line.variables::text NOT IN ('{}', 'null');

UPDATE kds_work_order_material_lines AS line
SET variables = lariv_migrate_formula_values(
    line.variables::jsonb, component.variables::jsonb, COALESCE(line.extra_data::jsonb, '{}'::jsonb)
)
FROM work_order_components AS component
WHERE component.id = line.component_id
  AND line.variables IS NOT NULL
  AND line.variables::text NOT IN ('{}', 'null');

UPDATE draft_work_order_machine_lines AS line
SET variables = lariv_migrate_formula_values(
    line.variables::jsonb, machine.variables::jsonb, '{}'::jsonb
)
FROM machinery_machines AS machine
WHERE machine.id = line.machine_id
  AND line.variables IS NOT NULL
  AND line.variables::text NOT IN ('{}', 'null');

UPDATE kds_quotation_machine_lines AS line
SET variables = lariv_migrate_formula_values(
    line.variables::jsonb, machine.variables::jsonb, '{}'::jsonb
)
FROM machinery_machines AS machine
WHERE machine.id = line.machine_id
  AND line.variables IS NOT NULL
  AND line.variables::text NOT IN ('{}', 'null');

UPDATE kds_work_order_machine_lines AS line
SET variables = lariv_migrate_formula_values(
    line.variables::jsonb, machine.variables::jsonb, '{}'::jsonb
)
FROM machinery_machines AS machine
WHERE machine.id = line.machine_id
  AND line.variables IS NOT NULL
  AND line.variables::text NOT IN ('{}', 'null');
"#;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        conn.execute_unprepared(ENSURE_FORMULA_MEASURE).await?;
        conn.execute_unprepared(MIGRATE).await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
