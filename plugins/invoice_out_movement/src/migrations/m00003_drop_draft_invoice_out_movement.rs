//! Draft invoices no longer store an out movement.

use lariv_core::db::migration_sql::is_postgres;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden, Clone, Copy)]
enum Col {
    StockMovementOutId,
}

#[derive(Iden, Clone, Copy)]
enum DraftInvoices {
    #[iden = "draft_invoices"]
    Table,
}

#[derive(Iden, Clone, Copy)]
enum MovementsOut {
    #[iden = "inventory_stock_movements_out"]
    Table,
    Id,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        drop_column(manager).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        add_column(manager).await
    }
}

async fn add_column(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .alter_table(
            Table::alter()
                .table(DraftInvoices::Table)
                .add_column(ColumnDef::new(Col::StockMovementOutId).big_integer().null())
                .to_owned(),
        )
        .await?;
    manager
        .create_index(
            Index::create()
                .if_not_exists()
                .name("idx_draft_invoices_stock_movement_out_id")
                .table(DraftInvoices::Table)
                .col(Col::StockMovementOutId)
                .to_owned(),
        )
        .await?;
    if is_postgres(manager) {
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_draft_invoices_stock_movement_out_id")
                    .from(DraftInvoices::Table, Col::StockMovementOutId)
                    .to(MovementsOut::Table, MovementsOut::Id)
                    .on_delete(ForeignKeyAction::Restrict)
                    .on_update(ForeignKeyAction::Cascade)
                    .to_owned(),
            )
            .await?;
    }
    Ok(())
}

async fn drop_column(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    if is_postgres(manager) {
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("fk_draft_invoices_stock_movement_out_id")
                    .table(DraftInvoices::Table)
                    .to_owned(),
            )
            .await?;
    }
    manager
        .drop_index(
            Index::drop()
                .name("idx_draft_invoices_stock_movement_out_id")
                .table(DraftInvoices::Table)
                .to_owned(),
        )
        .await?;
    manager
        .alter_table(
            Table::alter()
                .table(DraftInvoices::Table)
                .drop_column(Col::StockMovementOutId)
                .to_owned(),
        )
        .await
}
