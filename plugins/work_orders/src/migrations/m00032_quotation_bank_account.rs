//! Bank-details text on quotations, and the default copied onto new ones.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum KdsQuotations {
    Table,
    BankAccount,
}

#[derive(DeriveIden)]
enum WorkOrdersPreferences {
    Table,
    DefaultBankAccount,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(KdsQuotations::Table)
                    .add_column_if_not_exists(
                        ColumnDef::new(KdsQuotations::BankAccount).text().null(),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(WorkOrdersPreferences::Table)
                    .add_column_if_not_exists(
                        ColumnDef::new(WorkOrdersPreferences::DefaultBankAccount)
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
                    .table(KdsQuotations::Table)
                    .drop_column(KdsQuotations::BankAccount)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(WorkOrdersPreferences::Table)
                    .drop_column(WorkOrdersPreferences::DefaultBankAccount)
                    .to_owned(),
            )
            .await
    }
}
