use sea_orm_migration::prelude::*;

use super::DeliveryTag;

mod m00001_delivery_init;
mod m00002_challan_transport_and_terms;
mod m00003_move_challans_to_stock_movements;

#[derive(Clone, Copy, Default)]
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m00001_delivery_init::Migration),
            Box::new(m00002_challan_transport_and_terms::Migration),
            Box::new(m00003_move_challans_to_stock_movements::Migration),
        ]
    }
}

lariv_core::define_register_migrations! {
    plugin: DeliveryTag;
    migrator: Migrator;
}
