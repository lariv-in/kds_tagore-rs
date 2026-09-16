use sea_orm_migration::prelude::*;

use super::WorkOrdersTag;

mod m00001_work_orders_init;
mod m00002_seed_standard_shapes;
mod m00003_component_fixed_variables;
mod m00004_draft_work_orders;
mod m00005_remove_status_from_draft_work_orders;
mod m00006_draft_work_order_material_lines;
mod m00007_draft_work_order_machine_lines;
mod m00008_work_orders_preferences;

#[derive(Clone, Copy, Default)]
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m00001_work_orders_init::Migration),
            Box::new(m00002_seed_standard_shapes::Migration),
            Box::new(m00003_component_fixed_variables::Migration),
            Box::new(m00004_draft_work_orders::Migration),
            Box::new(m00005_remove_status_from_draft_work_orders::Migration),
            Box::new(m00006_draft_work_order_material_lines::Migration),
            Box::new(m00007_draft_work_order_machine_lines::Migration),
            Box::new(m00008_work_orders_preferences::Migration),
        ]
    }
}

lariv_rs::define_register_migrations! {
    plugin: WorkOrdersTag;
    migrator: Migrator;
}
