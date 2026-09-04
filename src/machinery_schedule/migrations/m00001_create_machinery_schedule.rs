use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum MachineryMachines {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    Name,
}

#[derive(DeriveIden)]
enum MachineryJobs {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    Name,
    Duration,
    Progress,
    Order,
    Remarks,
}

#[derive(DeriveIden)]
enum MachineryCompletedJobs {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    JobId,
    CompletedAt,
}

#[derive(DeriveIden)]
enum MachineryJobMachines {
    Table,
    JobId,
    MachineId,
}

#[derive(DeriveIden)]
enum MachineryJobFiles {
    Table,
    JobId,
    VNodeId,
}

#[derive(DeriveIden)]
enum FilesystemNodes {
    Table,
    Id,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(MachineryMachines::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(MachineryMachines::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(MachineryMachines::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(MachineryMachines::UpdatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(MachineryMachines::Name).text().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(MachineryJobs::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(MachineryJobs::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(MachineryJobs::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(MachineryJobs::UpdatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(MachineryJobs::Name).text().not_null())
                    .col(
                        ColumnDef::new(MachineryJobs::Duration)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(MachineryJobs::Progress)
                            .small_integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(MachineryJobs::Order).big_integer().not_null())
                    .col(ColumnDef::new(MachineryJobs::Remarks).text().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(MachineryCompletedJobs::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(MachineryCompletedJobs::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(MachineryCompletedJobs::CreatedAt)
                            .timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(MachineryCompletedJobs::UpdatedAt)
                            .timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(MachineryCompletedJobs::JobId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(MachineryCompletedJobs::CompletedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_machinery_completed_jobs_job_id")
                            .from(MachineryCompletedJobs::Table, MachineryCompletedJobs::JobId)
                            .to(MachineryJobs::Table, MachineryJobs::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("uix_machinery_completed_jobs_job_id")
                    .table(MachineryCompletedJobs::Table)
                    .col(MachineryCompletedJobs::JobId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(MachineryJobMachines::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(MachineryJobMachines::JobId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(MachineryJobMachines::MachineId)
                            .big_integer()
                            .not_null(),
                    )
                    .primary_key(
                        Index::create()
                            .col(MachineryJobMachines::JobId)
                            .col(MachineryJobMachines::MachineId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_machinery_job_machines_job_id")
                            .from(MachineryJobMachines::Table, MachineryJobMachines::JobId)
                            .to(MachineryJobs::Table, MachineryJobs::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_machinery_job_machines_machine_id")
                            .from(MachineryJobMachines::Table, MachineryJobMachines::MachineId)
                            .to(MachineryMachines::Table, MachineryMachines::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(MachineryJobFiles::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(MachineryJobFiles::JobId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(MachineryJobFiles::VNodeId)
                            .big_integer()
                            .not_null(),
                    )
                    .primary_key(
                        Index::create()
                            .col(MachineryJobFiles::JobId)
                            .col(MachineryJobFiles::VNodeId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_machinery_job_files_job_id")
                            .from(MachineryJobFiles::Table, MachineryJobFiles::JobId)
                            .to(MachineryJobs::Table, MachineryJobs::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_machinery_job_files_v_node_id")
                            .from(MachineryJobFiles::Table, MachineryJobFiles::VNodeId)
                            .to(FilesystemNodes::Table, FilesystemNodes::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(MachineryJobFiles::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(MachineryJobMachines::Table).to_owned())
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(MachineryCompletedJobs::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(Table::drop().table(MachineryJobs::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(MachineryMachines::Table).to_owned())
            .await
    }
}
