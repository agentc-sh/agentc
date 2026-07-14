// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use sea_orm_migration::prelude::{sea_orm::DbBackend, *};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(StateSnapshot::Table)
                    .rename_column(StateSnapshot::ModelOverride, StateSnapshot::Model)
                    .to_owned(),
            )
            .await?;

        manager
            .get_connection()
            .execute_unprepared(match manager.get_database_backend() {
                DbBackend::Postgres => {
                    r#"UPDATE "state_snapshot"
SET "model" = jsonb_build_object('override', "model")
WHERE "model" IS NOT NULL"#
                }
                DbBackend::Sqlite => {
                    r#"UPDATE "state_snapshot"
SET "model" = json_object('override', json("model"))
WHERE "model" IS NOT NULL"#
                }
                DbBackend::MySql => {
                    r#"UPDATE `state_snapshot`
SET `model` = JSON_OBJECT('override', `model`)
WHERE `model` IS NOT NULL"#
                }
            })
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(match manager.get_database_backend() {
                DbBackend::Postgres => {
                    r#"UPDATE "state_snapshot"
SET "model" = "model" -> 'override'
WHERE "model" IS NOT NULL"#
                }
                DbBackend::Sqlite => {
                    r#"UPDATE "state_snapshot"
SET "model" = json_extract("model", '$.override')
WHERE "model" IS NOT NULL"#
                }
                DbBackend::MySql => {
                    r#"UPDATE `state_snapshot`
SET `model` = JSON_EXTRACT(`model`, '$.override')
WHERE `model` IS NOT NULL"#
                }
            })
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(StateSnapshot::Table)
                    .rename_column(StateSnapshot::Model, StateSnapshot::ModelOverride)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum StateSnapshot {
    Table,
    ModelOverride,
    Model,
}
