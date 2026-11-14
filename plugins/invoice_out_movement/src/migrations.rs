use frunk::{HCons, hlist::HList};
use lariv_core::migration::{
    CollectForeignKeys, CollectMigrations, MigrationCapability, MigrationRegistrar,
    RegisteredMigrator,
};
use lariv_core::tag::Tagged;
use sea_orm_migration::prelude::*;

use super::InvoiceOutMovementTag;

mod m00001_invoice_stock_movement_out;
mod m00002_drop_posted_draft_copy_trigger;
mod m00003_drop_draft_invoice_out_movement;

#[derive(Clone, Copy, Default)]
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m00001_invoice_stock_movement_out::Migration),
            Box::new(m00002_drop_posted_draft_copy_trigger::Migration),
            Box::new(m00003_drop_draft_invoice_out_movement::Migration),
        ]
    }
}

#[derive(Clone, Copy, Default)]
pub struct Hook;

impl<M> MigrationRegistrar<M> for Hook
where
    M: HList + Clone + CollectMigrations + CollectForeignKeys + Send,
{
    // Concrete cons cell, matching `define_register_migrations!`. An opaque
    // associated type drops `CollectForeignKeys` and stops later plugins mounting.
    type Output = HCons<Tagged<InvoiceOutMovementTag, RegisteredMigrator<Migrator>>, M>;

    fn register_migrations(self, cap: MigrationCapability<M>) -> MigrationCapability<Self::Output> {
        crate::register_addons();
        cap.prepend::<InvoiceOutMovementTag, _>(RegisteredMigrator::new(Migrator, &[]))
    }
}
