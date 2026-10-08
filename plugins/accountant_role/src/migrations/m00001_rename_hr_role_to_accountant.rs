//! Rename the stored deployment role `hr` to `accountant`.

use lariv_core::db::migration_sql::exec_sql;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_sql(
            manager,
            "UPDATE users SET role = 'accountant' WHERE role = 'hr'",
        )
        .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_sql(
            manager,
            "UPDATE users SET role = 'hr' WHERE role = 'accountant'",
        )
        .await
    }
}
