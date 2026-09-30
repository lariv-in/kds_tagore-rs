use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum DeliveryPreferences {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    CompanyName,
    CompanyAddress,
    CompanyPhone,
    CompanyEmail,
    CompanyGstin,
    ChallanNumberFormat,
    DeliveryChallanPdfTemplate,
    LogoVnodeId,
    SignatureVnodeId,
}

#[derive(DeriveIden)]
enum DeliveryChallans {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    Date,
    CustomerId,
    ChallanNumber,
}

#[derive(DeriveIden)]
enum DeliveryChallanLines {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    DeliveryChallanId,
    SrNo,
    ProductId,
    QtyKind,
    QtyLength,
    QtyLengthUnit,
    QtyWeight,
    QtyNumber,
}

#[derive(DeriveIden)]
enum Customers {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Products {
    Table,
    Id,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(DeliveryPreferences::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(DeliveryPreferences::Id)
                            .big_integer()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(DeliveryPreferences::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(DeliveryPreferences::UpdatedAt).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(DeliveryPreferences::CompanyName)
                            .text()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(DeliveryPreferences::CompanyAddress)
                            .text()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(DeliveryPreferences::CompanyPhone)
                            .text()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(DeliveryPreferences::CompanyEmail)
                            .text()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(DeliveryPreferences::CompanyGstin)
                            .text()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(DeliveryPreferences::ChallanNumberFormat)
                            .text()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(DeliveryPreferences::DeliveryChallanPdfTemplate)
                            .text()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(DeliveryPreferences::LogoVnodeId)
                            .big_integer()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(DeliveryPreferences::SignatureVnodeId)
                            .big_integer()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(DeliveryChallans::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(DeliveryChallans::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(DeliveryChallans::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(DeliveryChallans::UpdatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(DeliveryChallans::Date).date().not_null())
                    .col(
                        ColumnDef::new(DeliveryChallans::CustomerId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(DeliveryChallans::ChallanNumber)
                            .text()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_delivery_challans_customer")
                            .from(DeliveryChallans::Table, DeliveryChallans::CustomerId)
                            .to(Customers::Table, Customers::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(DeliveryChallanLines::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(DeliveryChallanLines::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(DeliveryChallanLines::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(DeliveryChallanLines::UpdatedAt).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(DeliveryChallanLines::DeliveryChallanId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(DeliveryChallanLines::SrNo)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(DeliveryChallanLines::ProductId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(DeliveryChallanLines::QtyKind)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(DeliveryChallanLines::QtyLength)
                            .decimal_len(15, 6)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(DeliveryChallanLines::QtyLengthUnit)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(DeliveryChallanLines::QtyWeight)
                            .decimal_len(19, 6)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(DeliveryChallanLines::QtyNumber)
                            .big_integer()
                            .null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_delivery_challan_lines_challan")
                            .from(
                                DeliveryChallanLines::Table,
                                DeliveryChallanLines::DeliveryChallanId,
                            )
                            .to(DeliveryChallans::Table, DeliveryChallans::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_delivery_challan_lines_product")
                            .from(DeliveryChallanLines::Table, DeliveryChallanLines::ProductId)
                            .to(Products::Table, Products::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(DeliveryChallanLines::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(DeliveryChallans::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(DeliveryPreferences::Table).to_owned())
            .await?;
        Ok(())
    }
}
