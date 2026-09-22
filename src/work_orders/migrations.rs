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
mod m00009_rename_proforma_invoices_to_quotations;
mod m00010_quotation_component_material_lines;
mod m00011_quotation_number_format;
mod m00012_line_taxes;
mod m00013_default_line_taxes;
mod m00014_drop_quotation_work_order_id;
mod m00015_work_order_quotation_id;
mod m00016_machines_to_machinery_schedule;
mod m00017_create_work_orders;
mod m00018_job_source_doc_backfill;
mod m00019_formula_costing;

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
            Box::new(m00009_rename_proforma_invoices_to_quotations::Migration),
            Box::new(m00010_quotation_component_material_lines::Migration),
            Box::new(m00011_quotation_number_format::Migration),
            Box::new(m00012_line_taxes::Migration),
            Box::new(m00013_default_line_taxes::Migration),
            Box::new(m00014_drop_quotation_work_order_id::Migration),
            Box::new(m00015_work_order_quotation_id::Migration),
            Box::new(m00016_machines_to_machinery_schedule::Migration),
            Box::new(m00017_create_work_orders::Migration),
            Box::new(m00018_job_source_doc_backfill::Migration),
            Box::new(m00019_formula_costing::Migration),
        ]
    }
}

lariv_rs::define_register_migrations! {
    plugin: WorkOrdersTag;
    migrator: Migrator;
}
