use sea_orm_migration::prelude::*;

use super::AccountantRoleTag;

mod m00001_rename_hr_role_to_accountant;

#[derive(Clone, Copy, Default)]
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(m00001_rename_hr_role_to_accountant::Migration)]
    }
}

lariv_core::define_register_migrations! {
    plugin: AccountantRoleTag;
    migrator: Migrator;
}
