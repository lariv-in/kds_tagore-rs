use frunk::hlist::HList;
use lariv_core::migration::{CollectMigrations, MigrationCapability, MigrationRegistrar};
use sea_orm_migration::prelude::*;

use super::InvoiceOutMovementTag;

mod m00001_invoice_stock_movement_out;

#[derive(Clone, Copy, Default)]
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(m00001_invoice_stock_movement_out::Migration)]
    }
}

#[derive(Clone, Copy, Default)]
pub struct Hook;

impl<M> MigrationRegistrar<M> for Hook
where
    M: HList + Clone + CollectMigrations + Send,
{
    type Output = impl HList + CollectMigrations + Clone + Send;

    fn register_migrations(self, cap: MigrationCapability<M>) -> MigrationCapability<Self::Output> {
        crate::register_addons();
        cap.prepend::<InvoiceOutMovementTag, _>(Migrator)
    }
}
