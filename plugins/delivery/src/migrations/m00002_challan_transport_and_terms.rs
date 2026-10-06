use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum DeliveryChallans {
    Table,
    VehicleNo,
    EwayBill,
}

#[derive(DeriveIden)]
enum DeliveryPreferences {
    Table,
    TermsAndConditions,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(DeliveryChallans::Table)
                    .add_column_if_not_exists(
                        ColumnDef::new(DeliveryChallans::VehicleNo).text().null(),
                    )
                    .add_column_if_not_exists(
                        ColumnDef::new(DeliveryChallans::EwayBill).text().null(),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(DeliveryPreferences::Table)
                    .add_column_if_not_exists(
                        ColumnDef::new(DeliveryPreferences::TermsAndConditions)
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
                    .table(DeliveryPreferences::Table)
                    .drop_column(DeliveryPreferences::TermsAndConditions)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(DeliveryChallans::Table)
                    .drop_column(DeliveryChallans::VehicleNo)
                    .drop_column(DeliveryChallans::EwayBill)
                    .to_owned(),
            )
            .await
    }
}
