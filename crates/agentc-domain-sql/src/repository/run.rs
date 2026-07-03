// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use std::str::FromStr;
use uuid::Uuid;

use agentc_database::{
    connection::ConnectionContext,
    orm::{ActiveValue, ColumnTrait, EntityTrait, Iden, QueryFilter, QueryTrait},
    paginate::CursorPaginatorExt,
    query::OnConflict,
};
use agentc_domain::{
    repository::run::{
        errors::RunRepoError,
        params::{DeleteRunParams, FindRunParams},
        traits::RunRepository,
    },
    types::{Page, Run, RunStatus},
};

pub mod models {
    pub mod run {
        use agentc_database::{
            errors::DatabaseError,
            orm::prelude::*,
            paginate::{CursorValue, ExtractCursorValue},
        };
        use async_trait::async_trait;
        use chrono::{DateTime, Utc};
        use uuid::Uuid;

        #[derive(Debug, Clone, PartialEq, Eq, DeriveEntityModel)]
        #[sea_orm(table_name = "run")]
        pub struct Model {
            #[sea_orm(primary_key, auto_increment = false)]
            pub id: Uuid,
            pub tenant_id: String,
            pub session_id: Uuid,
            pub status: String,
            pub current_node: Option<String>,
            pub latest_checkpoint_id: Option<Uuid>,
            pub last_interrupted_checkpoint_id: Option<Uuid>,
            pub created_at: DateTime<Utc>,
            pub updated_at: DateTime<Utc>,
        }

        #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
        pub enum Relation {}

        #[async_trait]
        impl ActiveModelBehavior for ActiveModel {}

        impl ExtractCursorValue for Model {
            fn extract_cursor_value(&self, field_name: &str) -> Result<CursorValue, DatabaseError> {
                match field_name {
                    "id" => Ok(CursorValue::Uuid(Some(self.id))),
                    "tenant_id" => Ok(CursorValue::String(Some(self.tenant_id.clone()))),
                    "session_id" => Ok(CursorValue::Uuid(Some(self.session_id))),
                    "status" => Ok(CursorValue::String(Some(self.status.clone()))),
                    "current_node" => Ok(CursorValue::String(self.current_node.clone())),
                    "latest_checkpoint_id" => Ok(CursorValue::Uuid(self.latest_checkpoint_id)),
                    "last_interrupted_checkpoint_id" => {
                        Ok(CursorValue::Uuid(self.last_interrupted_checkpoint_id))
                    }
                    "created_at" => Ok(CursorValue::DateTime(Some(self.created_at))),
                    "updated_at" => Ok(CursorValue::DateTime(Some(self.updated_at))),
                    _ => Err(DatabaseError::UnknownFieldName(field_name.to_string())),
                }
            }
        }
    }
}

pub struct SqlRunRepository<'a> {
    ctx: ConnectionContext<'a>,
}

impl<'a> SqlRunRepository<'a> {
    pub fn new(ctx: ConnectionContext<'a>) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl<'a> RunRepository for SqlRunRepository<'a> {
    async fn save(&self, runs: Vec<Run>) -> Result<Vec<Run>, RunRepoError> {
        if runs.is_empty() {
            return Ok(vec![]);
        }

        models::run::Entity::insert_many(
            runs.into_iter()
                .map(|run| models::run::ActiveModel {
                    id: ActiveValue::set(run.id),
                    tenant_id: ActiveValue::set(run.tenant_id),
                    session_id: ActiveValue::set(run.session_id),
                    status: ActiveValue::set(run.status.to_string()),
                    current_node: ActiveValue::set(run.current_node),
                    latest_checkpoint_id: ActiveValue::set(run.latest_checkpoint_id),
                    last_interrupted_checkpoint_id: ActiveValue::set(
                        run.last_interrupted_checkpoint_id,
                    ),
                    created_at: ActiveValue::set(run.created_at),
                    updated_at: ActiveValue::set(run.updated_at),
                }),
        )
        .on_conflict(
            OnConflict::columns([models::run::Column::Id, models::run::Column::TenantId])
                .update_columns([
                    models::run::Column::Status,
                    models::run::Column::CurrentNode,
                    models::run::Column::LatestCheckpointId,
                    models::run::Column::LastInterruptedCheckpointId,
                    models::run::Column::UpdatedAt,
                ])
                .to_owned(),
        )
        .exec_with_returning_many(&self.ctx)
        .await
        .map_err(RunRepoError::sourced_storage)
        .map(|models| {
            models
                .into_iter()
                .map(|run| Run {
                    id: run.id,
                    tenant_id: run.tenant_id,
                    session_id: run.session_id,
                    status: RunStatus::from_str(&run.status).unwrap_or(RunStatus::Failed),
                    current_node: run.current_node,
                    latest_checkpoint_id: run.latest_checkpoint_id,
                    last_interrupted_checkpoint_id: run.last_interrupted_checkpoint_id,
                    created_at: run.created_at,
                    updated_at: run.updated_at,
                })
                .collect()
        })
    }

    async fn get(&self, tenant_id: &str, id: Uuid) -> Result<Option<Run>, RunRepoError> {
        models::run::Entity::find()
            .filter(models::run::Column::TenantId.eq(tenant_id.to_string()))
            .filter(models::run::Column::Id.eq(id))
            .one(&self.ctx)
            .await
            .map_err(RunRepoError::sourced_storage)
            .map(|opt| {
                opt.map(|run| Run {
                    id: run.id,
                    tenant_id: run.tenant_id,
                    session_id: run.session_id,
                    status: RunStatus::from_str(&run.status).unwrap_or(RunStatus::Failed),
                    current_node: run.current_node,
                    latest_checkpoint_id: run.latest_checkpoint_id,
                    last_interrupted_checkpoint_id: run.last_interrupted_checkpoint_id,
                    created_at: run.created_at,
                    updated_at: run.updated_at,
                })
            })
    }

    async fn find(&self, params: FindRunParams) -> Result<Page<Run>, RunRepoError> {
        models::run::Entity::find()
            .apply_if(params.tenant_ids, |query, val| {
                query.filter(models::run::Column::TenantId.is_in(val))
            })
            .apply_if(params.ids, |query, val| query.filter(models::run::Column::Id.is_in(val)))
            .apply_if(params.session_ids, |query, val| {
                query.filter(models::run::Column::SessionId.is_in(val))
            })
            .apply_if(params.statuses, |query, val| {
                query.filter(
                    models::run::Column::Status.is_in(
                        val.into_iter()
                            .map(|s| s.to_string())
                            .collect::<Vec<_>>(),
                    ),
                )
            })
            .apply_if(params.created_before, |query, val| {
                query.filter(models::run::Column::CreatedAt.lte(val))
            })
            .apply_if(params.created_after, |query, val| {
                query.filter(models::run::Column::CreatedAt.gte(val))
            })
            .apply_if(params.updated_before, |query, val| {
                query.filter(models::run::Column::UpdatedAt.lte(val))
            })
            .apply_if(params.updated_after, |query, val| {
                query.filter(models::run::Column::UpdatedAt.gte(val))
            })
            .cursor_paginate()
            .per_page(params.per_page)
            .cursor(params.page)
            .sort_desc(models::run::Column::CreatedAt.to_string())
            .execute(&self.ctx)
            .await
            .map_err(RunRepoError::sourced_storage)
            .map(|page| Page {
                items: page
                    .items
                    .into_iter()
                    .map(|run| Run {
                        id: run.id,
                        tenant_id: run.tenant_id,
                        session_id: run.session_id,
                        status: RunStatus::from_str(&run.status).unwrap_or(RunStatus::Failed),
                        current_node: run.current_node,
                        latest_checkpoint_id: run.latest_checkpoint_id,
                        last_interrupted_checkpoint_id: run.last_interrupted_checkpoint_id,
                        created_at: run.created_at,
                        updated_at: run.updated_at,
                    })
                    .collect(),
                count: page.count,
                next_page: page.next_page,
            })
    }

    async fn delete(&self, params: DeleteRunParams) -> Result<(), RunRepoError> {
        models::run::Entity::delete_many()
            .filter(models::run::Column::TenantId.eq(params.tenant_id))
            .filter(models::run::Column::Id.is_in(params.ids))
            .exec(&self.ctx)
            .await
            .map_err(RunRepoError::sourced_storage)
            .map(|_| ())
    }
}
