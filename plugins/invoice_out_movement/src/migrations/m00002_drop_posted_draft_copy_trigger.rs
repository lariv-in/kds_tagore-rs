//! Posted invoices no longer reference drafts; copy movement id in app code via on_posted.

use lariv_core::db::migration_sql::{exec_sql, is_postgres};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if is_postgres(manager) {
            exec_each(
                manager,
                &[
                    "DROP TRIGGER IF EXISTS posted_invoices_copy_stock_movement_out ON posted_invoices",
                    "DROP FUNCTION IF EXISTS kds_copy_posted_invoice_stock_movement_out()",
                ],
            )
            .await?;
        } else {
            exec_each(
                manager,
                &["DROP TRIGGER IF EXISTS posted_invoices_copy_stock_movement_out"],
            )
            .await?;
        }
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        // Not restored: draft_invoice_id on posted_invoices is gone after finance m00044.
        Ok(())
    }
}

async fn exec_each(manager: &SchemaManager<'_>, sqls: &[&str]) -> Result<(), DbErr> {
    for sql in sqls {
        exec_sql(manager, sql).await?;
    }
    Ok(())
}
