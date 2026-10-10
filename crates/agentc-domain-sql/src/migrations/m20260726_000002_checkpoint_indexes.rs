// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_index(
                Index::create()
                    .name("idx_checkpoint_record_tenant_session")
                    .table(CheckpointRecord::Table)
                    .col(CheckpointRecord::TenantId)
                    .col(CheckpointRecord::SessionId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_checkpoint_record_tenant_run")
                    .table(CheckpointRecord::Table)
                    .col(CheckpointRecord::TenantId)
                    .col(CheckpointRecord::RunId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("idx_checkpoint_record_tenant_run")
                    .table(CheckpointRecord::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_checkpoint_record_tenant_session")
                    .table(CheckpointRecord::Table)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum CheckpointRecord {
    Table,
    TenantId,
    SessionId,
    RunId,
}
