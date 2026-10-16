//! `stock_movement_out_id` on invoice headers, copied when a draft is posted or cancelled.

use lariv_core::db::migration_sql::{exec_sql, is_postgres};
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
enum PostedInvoices {
    #[iden = "posted_invoices"]
    Table,
}

#[derive(Iden, Clone, Copy)]
enum CancelledInvoices {
    #[iden = "cancelled_invoices"]
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
        add_column(manager, DraftInvoices::Table, "draft_invoices").await?;
        add_column(manager, PostedInvoices::Table, "posted_invoices").await?;
        add_column(manager, CancelledInvoices::Table, "cancelled_invoices").await?;
        if is_postgres(manager) {
            exec_each(
                manager,
                &[
                    "
                    CREATE OR REPLACE FUNCTION kds_copy_posted_invoice_stock_movement_out()
                    RETURNS trigger AS $$
                    BEGIN
                      IF NEW.stock_movement_out_id IS NULL THEN
                        SELECT d.stock_movement_out_id INTO NEW.stock_movement_out_id
                        FROM draft_invoices d
                        WHERE d.id = NEW.draft_invoice_id;
                      END IF;
                      RETURN NEW;
                    END;
                    $$ LANGUAGE plpgsql
                    ",
                    "
                    CREATE TRIGGER posted_invoices_copy_stock_movement_out
                    BEFORE INSERT ON posted_invoices
                    FOR EACH ROW
                    EXECUTE FUNCTION kds_copy_posted_invoice_stock_movement_out()
                    ",
                    "
                    CREATE OR REPLACE FUNCTION kds_copy_cancelled_invoice_stock_movement_out()
                    RETURNS trigger AS $$
                    BEGIN
                      IF NEW.stock_movement_out_id IS NULL THEN
                        SELECT p.stock_movement_out_id INTO NEW.stock_movement_out_id
                        FROM posted_invoices p
                        WHERE p.id = NEW.posted_invoice_id;
                      END IF;
                      RETURN NEW;
                    END;
                    $$ LANGUAGE plpgsql
                    ",
                    "
                    CREATE TRIGGER cancelled_invoices_copy_stock_movement_out
                    BEFORE INSERT ON cancelled_invoices
                    FOR EACH ROW
                    EXECUTE FUNCTION kds_copy_cancelled_invoice_stock_movement_out()
                    ",
                ],
            )
            .await?;
        } else {
            exec_each(
                manager,
                &[
                    "
                    CREATE TRIGGER posted_invoices_copy_stock_movement_out
                    AFTER INSERT ON posted_invoices
                    FOR EACH ROW
                    WHEN NEW.stock_movement_out_id IS NULL
                    BEGIN
                      UPDATE posted_invoices
                      SET stock_movement_out_id = (
                        SELECT stock_movement_out_id FROM draft_invoices WHERE id = NEW.draft_invoice_id
                      )
                      WHERE id = NEW.id;
                    END
                    ",
                    "
                    CREATE TRIGGER cancelled_invoices_copy_stock_movement_out
                    AFTER INSERT ON cancelled_invoices
                    FOR EACH ROW
                    WHEN NEW.stock_movement_out_id IS NULL
                    BEGIN
                      UPDATE cancelled_invoices
                      SET stock_movement_out_id = (
                        SELECT stock_movement_out_id FROM posted_invoices WHERE id = NEW.posted_invoice_id
                      )
                      WHERE id = NEW.id;
                    END
                    ",
                ],
            )
            .await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if is_postgres(manager) {
            exec_each(
                manager,
                &[
                    "DROP TRIGGER IF EXISTS posted_invoices_copy_stock_movement_out ON posted_invoices",
                    "DROP TRIGGER IF EXISTS cancelled_invoices_copy_stock_movement_out ON cancelled_invoices",
                    "DROP FUNCTION IF EXISTS kds_copy_posted_invoice_stock_movement_out()",
                    "DROP FUNCTION IF EXISTS kds_copy_cancelled_invoice_stock_movement_out()",
                ],
            )
            .await?;
        } else {
            exec_each(
                manager,
                &[
                    "DROP TRIGGER IF EXISTS posted_invoices_copy_stock_movement_out",
                    "DROP TRIGGER IF EXISTS cancelled_invoices_copy_stock_movement_out",
                ],
            )
            .await?;
        }
        drop_column(manager, DraftInvoices::Table, "draft_invoices").await?;
        drop_column(manager, PostedInvoices::Table, "posted_invoices").await?;
        drop_column(manager, CancelledInvoices::Table, "cancelled_invoices").await?;
        Ok(())
    }
}

async fn exec_each(manager: &SchemaManager<'_>, statements: &[&str]) -> Result<(), DbErr> {
    for sql in statements {
        exec_sql(manager, sql).await?;
    }
    Ok(())
}

async fn add_column(
    manager: &SchemaManager<'_>,
    table: impl Iden + Copy + 'static,
    prefix: &str,
) -> Result<(), DbErr> {
    manager
        .alter_table(
            Table::alter()
                .table(table)
                .add_column(ColumnDef::new(Col::StockMovementOutId).big_integer().null())
                .to_owned(),
        )
        .await?;
    manager
        .create_index(
            Index::create()
                .if_not_exists()
                .name(format!("idx_{prefix}_stock_movement_out_id"))
                .table(table)
                .col(Col::StockMovementOutId)
                .to_owned(),
        )
        .await?;
    // SQLite cannot add a foreign key to an existing table.
    if is_postgres(manager) {
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name(format!("fk_{prefix}_stock_movement_out_id"))
                    .from(table, Col::StockMovementOutId)
                    .to(MovementsOut::Table, MovementsOut::Id)
                    .on_delete(ForeignKeyAction::Restrict)
                    .on_update(ForeignKeyAction::Cascade)
                    .to_owned(),
            )
            .await?;
    }
    Ok(())
}

async fn drop_column(
    manager: &SchemaManager<'_>,
    table: impl Iden + Copy + 'static,
    prefix: &str,
) -> Result<(), DbErr> {
    if is_postgres(manager) {
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name(format!("fk_{prefix}_stock_movement_out_id"))
                    .table(table)
                    .to_owned(),
            )
            .await?;
    }
    manager
        .drop_index(
            Index::drop()
                .name(format!("idx_{prefix}_stock_movement_out_id"))
                .table(table)
                .to_owned(),
        )
        .await?;
    manager
        .alter_table(
            Table::alter()
                .table(table)
                .drop_column(Col::StockMovementOutId)
                .to_owned(),
        )
        .await
}
