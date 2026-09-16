use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum WorkOrdersPreferences {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    DraftWorkOrderPdfTemplate,
    ProformaInvoicePdfTemplate,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(WorkOrdersPreferences::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(WorkOrdersPreferences::Id)
                            .big_integer()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(WorkOrdersPreferences::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(WorkOrdersPreferences::UpdatedAt).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(WorkOrdersPreferences::DraftWorkOrderPdfTemplate)
                            .text()
                            .null()
                            .default(""),
                    )
                    .col(
                        ColumnDef::new(WorkOrdersPreferences::ProformaInvoicePdfTemplate)
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
            .drop_table(Table::drop().table(WorkOrdersPreferences::Table).to_owned())
            .await
    }
}