use sea_orm_migration::prelude::*;

use super::KdsWebsiteSeedTag;

mod m00001_invoice_bank_account_default;

#[derive(Clone, Copy, Default)]
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(m00001_invoice_bank_account_default::Migration)]
    }
}

lariv_core::define_register_migrations! {
    plugin: KdsWebsiteSeedTag;
    migrator: Migrator;
}
