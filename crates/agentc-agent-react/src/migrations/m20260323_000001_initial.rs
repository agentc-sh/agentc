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
            .create_table(
                Table::create()
                    .table(Message::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Message::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Message::TenantId)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Message::SessionId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Message::RunId)
                            .uuid()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(Message::Role)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Message::Content)
                            .text()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(Message::Data)
                            .json_binary()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(Message::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("unique_message_id_tenant_id")
                    .table(Message::Table)
                    .col(Message::Id)
                    .col(Message::TenantId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_message_session_id")
                    .table(Message::Table)
                    .col(Message::SessionId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_message_run_id")
                    .table(Message::Table)
                    .col(Message::RunId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_message_tenant_id")
                    .table(Message::Table)
                    .col(Message::TenantId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(StateSnapshot::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(StateSnapshot::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(StateSnapshot::TenantId)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(StateSnapshot::SessionId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(StateSnapshot::RunId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(StateSnapshot::CheckpointId)
                            .uuid()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(StateSnapshot::ModelOverride)
                            .json_binary()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(StateSnapshot::CapabilityOverride)
                            .json_binary()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(StateSnapshot::Tools)
                            .json_binary()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(StateSnapshot::ContextVars)
                            .json_binary()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(StateSnapshot::Context)
                            .json_binary()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(StateSnapshot::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(StateSnapshot::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("unique_state_snapshot_id_tenant_id")
                    .table(StateSnapshot::Table)
                    .col(StateSnapshot::Id)
                    .col(StateSnapshot::TenantId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_state_snapshot_session_id")
                    .table(StateSnapshot::Table)
                    .col(StateSnapshot::SessionId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_state_snapshot_run_id")
                    .table(StateSnapshot::Table)
                    .col(StateSnapshot::RunId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_state_snapshot_checkpoint_id")
                    .table(StateSnapshot::Table)
                    .col(StateSnapshot::CheckpointId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_state_snapshot_tenant_id")
                    .table(StateSnapshot::Table)
                    .col(StateSnapshot::TenantId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}

#[derive(DeriveIden)]
enum Message {
    Table,
    Id,
    TenantId,
    SessionId,
    RunId,
    Role,
    Content,
    Data,
    CreatedAt,
}

#[derive(DeriveIden)]
enum StateSnapshot {
    Table,
    Id,
    TenantId,
    SessionId,
    RunId,
    CheckpointId,
    ModelOverride,
    CapabilityOverride,
    Tools,
    ContextVars,
    Context,
    CreatedAt,
    UpdatedAt,
}
