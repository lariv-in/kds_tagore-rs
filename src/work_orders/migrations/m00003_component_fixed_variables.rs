use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum WorkOrderComponents {
    Table,
    FixedVariables,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(WorkOrderComponents::Table)
                    .add_column_if_not_exists(
                        ColumnDef::new(WorkOrderComponents::FixedVariables)
                            .json()
                            .not_null()
                            .default("{}"),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(WorkOrderComponents::Table)
                    .drop_column(WorkOrderComponents::FixedVariables)
                    .to_owned(),
            )
            .await
    }
}
