use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum WorkOrdersPreferences {
    Table,
    QuotationNumberFormat,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(WorkOrdersPreferences::Table)
                    .add_column_if_not_exists(
                        ColumnDef::new(WorkOrdersPreferences::QuotationNumberFormat)
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
                    .drop_column(WorkOrdersPreferences::QuotationNumberFormat)
                    .to_owned(),
            )
            .await
    }
}
