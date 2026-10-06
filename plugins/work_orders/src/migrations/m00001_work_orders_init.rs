use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum WorkOrderMaterials {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    Name,
    Density,
}

#[derive(DeriveIden)]
enum WorkOrderMaterialRates {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    MaterialId,
    RateDecimal,
    Datetime,
}

#[derive(DeriveIden)]
enum WorkOrderShapes {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    Name,
    OpenscadCode,
    VariableNames,
}

#[derive(DeriveIden)]
enum WorkOrderComponents {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    Name,
    ShapeId,
    MaterialId,
}

#[derive(DeriveIden)]
enum WorkOrderMachines {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    Name,
    RateDecimal,
}

#[derive(DeriveIden)]
enum DraftWorkOrders {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    OrderNumber,
    CustomerId,
    Status,
}

#[derive(DeriveIden)]
enum DraftWorkOrderLines {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    DraftWorkOrderId,
    ComponentId,
    Variables,
    Quantity,
    UnitWeight,
    MaterialRate,
    FinalCost,
    ExtraData,
}

type WorkOrders = DraftWorkOrders;
type WorkOrderLines = DraftWorkOrderLines;

#[derive(DeriveIden)]
enum KdsProformaInvoices {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    Date,
    CustomerId,
    InvoiceNumber,
    WorkOrderId,
}

#[derive(DeriveIden)]
enum KdsProformaInvoiceMachineLines {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    InvoiceId,
    MachineId,
    Name,
    TimeUsed,
    RateDecimal,
}

#[derive(DeriveIden)]
enum KdsProformaInvoiceMaterialLines {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    InvoiceId,
    MaterialId,
    Name,
    RateDecimal,
    QtyDecimal,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 1. Materials
        manager
            .create_table(
                Table::create()
                    .table(WorkOrderMaterials::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(WorkOrderMaterials::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(WorkOrderMaterials::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(WorkOrderMaterials::UpdatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(WorkOrderMaterials::Name).text().not_null())
                    .col(
                        ColumnDef::new(WorkOrderMaterials::Density)
                            .double()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        // 2. Material Rates
        manager
            .create_table(
                Table::create()
                    .table(WorkOrderMaterialRates::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(WorkOrderMaterialRates::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(WorkOrderMaterialRates::CreatedAt)
                            .timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(WorkOrderMaterialRates::UpdatedAt)
                            .timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(WorkOrderMaterialRates::MaterialId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WorkOrderMaterialRates::RateDecimal)
                            .decimal_len(16, 4)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WorkOrderMaterialRates::Datetime)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_work_order_rates_material")
                            .from(
                                WorkOrderMaterialRates::Table,
                                WorkOrderMaterialRates::MaterialId,
                            )
                            .to(WorkOrderMaterials::Table, WorkOrderMaterials::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // 3. Shapes
        manager
            .create_table(
                Table::create()
                    .table(WorkOrderShapes::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(WorkOrderShapes::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(WorkOrderShapes::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(WorkOrderShapes::UpdatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(WorkOrderShapes::Name).text().not_null())
                    .col(
                        ColumnDef::new(WorkOrderShapes::OpenscadCode)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WorkOrderShapes::VariableNames)
                            .json()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        // 4. Components
        manager
            .create_table(
                Table::create()
                    .table(WorkOrderComponents::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(WorkOrderComponents::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(WorkOrderComponents::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(WorkOrderComponents::UpdatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(WorkOrderComponents::Name).text().not_null())
                    .col(
                        ColumnDef::new(WorkOrderComponents::ShapeId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WorkOrderComponents::MaterialId)
                            .big_integer()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_work_order_components_shape")
                            .from(WorkOrderComponents::Table, WorkOrderComponents::ShapeId)
                            .to(WorkOrderShapes::Table, WorkOrderShapes::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_work_order_components_material")
                            .from(WorkOrderComponents::Table, WorkOrderComponents::MaterialId)
                            .to(WorkOrderMaterials::Table, WorkOrderMaterials::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        // 5. Machines
        manager
            .create_table(
                Table::create()
                    .table(WorkOrderMachines::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(WorkOrderMachines::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(WorkOrderMachines::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(WorkOrderMachines::UpdatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(WorkOrderMachines::Name).text().not_null())
                    .col(
                        ColumnDef::new(WorkOrderMachines::RateDecimal)
                            .decimal_len(16, 4)
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        // 6. Work Orders
        manager
            .create_table(
                Table::create()
                    .table(WorkOrders::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(WorkOrders::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(WorkOrders::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(WorkOrders::UpdatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(WorkOrders::OrderNumber).text().not_null())
                    .col(
                        ColumnDef::new(WorkOrders::CustomerId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WorkOrders::Status)
                            .text()
                            .not_null()
                            .default("Draft"),
                    )
                    .to_owned(),
            )
            .await?;

        // 6b. Draft Work Order Lines
        manager
            .create_table(
                Table::create()
                    .table(DraftWorkOrderLines::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(DraftWorkOrderLines::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(DraftWorkOrderLines::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(DraftWorkOrderLines::UpdatedAt).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(DraftWorkOrderLines::DraftWorkOrderId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(DraftWorkOrderLines::ComponentId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(DraftWorkOrderLines::Variables)
                            .json()
                            .not_null()
                            .default("{}"),
                    )
                    .col(
                        ColumnDef::new(DraftWorkOrderLines::Quantity)
                            .decimal_len(16, 4)
                            .not_null()
                            .default(1),
                    )
                    .col(
                        ColumnDef::new(DraftWorkOrderLines::UnitWeight)
                            .decimal_len(16, 4)
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(DraftWorkOrderLines::MaterialRate)
                            .decimal_len(16, 4)
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(DraftWorkOrderLines::FinalCost)
                            .decimal_len(16, 4)
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(DraftWorkOrderLines::ExtraData)
                            .json()
                            .not_null()
                            .default("{}"),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_draft_work_order_lines_draft_order")
                            .from(
                                DraftWorkOrderLines::Table,
                                DraftWorkOrderLines::DraftWorkOrderId,
                            )
                            .to(DraftWorkOrders::Table, DraftWorkOrders::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_draft_work_order_lines_component")
                            .from(DraftWorkOrderLines::Table, DraftWorkOrderLines::ComponentId)
                            .to(WorkOrderComponents::Table, WorkOrderComponents::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        // 7. KDS Proforma Invoices
        manager
            .create_table(
                Table::create()
                    .table(KdsProformaInvoices::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(KdsProformaInvoices::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(KdsProformaInvoices::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(KdsProformaInvoices::UpdatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(KdsProformaInvoices::Date).date().not_null())
                    .col(
                        ColumnDef::new(KdsProformaInvoices::CustomerId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(KdsProformaInvoices::InvoiceNumber)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(KdsProformaInvoices::WorkOrderId)
                            .big_integer()
                            .null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_kds_proforma_invoices_work_order")
                            .from(KdsProformaInvoices::Table, KdsProformaInvoices::WorkOrderId)
                            .to(WorkOrders::Table, WorkOrders::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await?;

        // 8. Machine Lines
        manager
            .create_table(
                Table::create()
                    .table(KdsProformaInvoiceMachineLines::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(KdsProformaInvoiceMachineLines::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(KdsProformaInvoiceMachineLines::CreatedAt)
                            .timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(KdsProformaInvoiceMachineLines::UpdatedAt)
                            .timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(KdsProformaInvoiceMachineLines::InvoiceId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(KdsProformaInvoiceMachineLines::MachineId)
                            .big_integer()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(KdsProformaInvoiceMachineLines::Name)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(KdsProformaInvoiceMachineLines::TimeUsed)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(KdsProformaInvoiceMachineLines::RateDecimal)
                            .decimal_len(16, 4)
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_kds_proforma_machine_lines_invoice")
                            .from(
                                KdsProformaInvoiceMachineLines::Table,
                                KdsProformaInvoiceMachineLines::InvoiceId,
                            )
                            .to(KdsProformaInvoices::Table, KdsProformaInvoices::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // 9. Material Lines
        manager
            .create_table(
                Table::create()
                    .table(KdsProformaInvoiceMaterialLines::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(KdsProformaInvoiceMaterialLines::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(KdsProformaInvoiceMaterialLines::CreatedAt)
                            .timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(KdsProformaInvoiceMaterialLines::UpdatedAt)
                            .timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(KdsProformaInvoiceMaterialLines::InvoiceId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(KdsProformaInvoiceMaterialLines::MaterialId)
                            .big_integer()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(KdsProformaInvoiceMaterialLines::Name)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(KdsProformaInvoiceMaterialLines::RateDecimal)
                            .decimal_len(16, 4)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(KdsProformaInvoiceMaterialLines::QtyDecimal)
                            .decimal_len(16, 4)
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_kds_proforma_material_lines_invoice")
                            .from(
                                KdsProformaInvoiceMaterialLines::Table,
                                KdsProformaInvoiceMaterialLines::InvoiceId,
                            )
                            .to(KdsProformaInvoices::Table, KdsProformaInvoices::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(KdsProformaInvoiceMaterialLines::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(KdsProformaInvoiceMachineLines::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(Table::drop().table(KdsProformaInvoices::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(WorkOrderLines::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(WorkOrders::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(WorkOrderMachines::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(WorkOrderComponents::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(WorkOrderShapes::Table).to_owned())
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(WorkOrderMaterialRates::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(Table::drop().table(WorkOrderMaterials::Table).to_owned())
            .await?;
        Ok(())
    }
}
