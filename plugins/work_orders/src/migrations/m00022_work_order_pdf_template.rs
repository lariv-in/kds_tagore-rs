//! Typst template preference for finalized (issued) work order PDFs.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum WorkOrdersPreferences {
    Table,
    WorkOrderPdfTemplate,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(WorkOrdersPreferences::Table)
                    .add_column_if_not_exists(
                        ColumnDef::new(WorkOrdersPreferences::WorkOrderPdfTemplate)
                            .text()
                            .null()
                            .default(""),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(WorkOrdersPreferences::Table)
                    .drop_column(WorkOrdersPreferences::WorkOrderPdfTemplate)
                    .to_owned(),
            )
            .await
    }
}
