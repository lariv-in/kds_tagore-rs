//! Quotation email subject and body templates on KDS Quotations preferences.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum WorkOrdersPreferences {
    Table,
    QuotationEmailSubject,
    QuotationEmailBody,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(WorkOrdersPreferences::Table)
                    .add_column_if_not_exists(
                        ColumnDef::new(WorkOrdersPreferences::QuotationEmailSubject)
                            .text()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(WorkOrdersPreferences::Table)
                    .add_column_if_not_exists(
                        ColumnDef::new(WorkOrdersPreferences::QuotationEmailBody)
                            .text()
                            .null(),
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
                    .drop_column(WorkOrdersPreferences::QuotationEmailSubject)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(WorkOrdersPreferences::Table)
                    .drop_column(WorkOrdersPreferences::QuotationEmailBody)
                    .to_owned(),
            )
            .await
    }
}
