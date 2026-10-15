use sea_orm_migration::prelude::*;

use super::MachineryScheduleTag;

mod m00001_create_machinery_schedule;
mod m00002_machine_hourly_rate;
mod m00003_job_source_doc;
mod m00004_machine_formula;
mod m00005_formula_measure;
mod m00006_formula_fixed_units;
mod m00007_decimal_unit_none;

#[derive(Clone, Copy, Default)]
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m00001_create_machinery_schedule::Migration),
            Box::new(m00002_machine_hourly_rate::Migration),
            Box::new(m00003_job_source_doc::Migration),
            Box::new(m00004_machine_formula::Migration),
            Box::new(m00005_formula_measure::Migration),
            Box::new(m00006_formula_fixed_units::Migration),
            Box::new(m00007_decimal_unit_none::Migration),
        ]
    }
}

lariv_core::define_register_migrations! {
    plugin: MachineryScheduleTag;
    migrator: Migrator;
}
