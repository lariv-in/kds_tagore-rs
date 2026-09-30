use sea_orm_migration::prelude::*;

use super::DeliveryTag;

mod m00001_delivery_init;
mod m00002_challan_transport_and_terms;

#[derive(Clone, Copy, Default)]
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m00001_delivery_init::Migration),
            Box::new(m00002_challan_transport_and_terms::Migration),
        ]
    }
}

lariv_rs::define_register_migrations! {
    plugin: DeliveryTag;
    migrator: Migrator;
}
