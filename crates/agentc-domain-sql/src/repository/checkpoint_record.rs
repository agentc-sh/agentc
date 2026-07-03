// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use uuid::Uuid;

use agentc_database::{
    connection::ConnectionContext,
    orm::{ActiveValue, ColumnTrait, EntityTrait, Iden, QueryFilter, QueryTrait},
    paginate::CursorPaginatorExt,
    query::OnConflict,
};
use agentc_domain::{
    repository::checkpoint_record::{
        errors::CheckpointRecordRepoError,
        params::{DeleteCheckpointRecordParams, FindCheckpointRecordParams},
        traits::CheckpointRecordRepository,
    },
    types::{CheckpointRecord, Page},
};

pub mod models {
    pub mod checkpoint_record {
        use agentc_database::{
            errors::DatabaseError,
            orm::prelude::*,
            paginate::{CursorValue, ExtractCursorValue},
        };
        use agentc_domain::types::{CheckpointReason, CheckpointRecord, RunStatus};
        use async_trait::async_trait;
        use chrono::{DateTime, Utc};
        use serde_json::Value;
        use std::str::FromStr;
        use uuid::Uuid;

        #[derive(Debug, Clone, PartialEq, Eq, DeriveEntityModel)]
        #[sea_orm(table_name = "checkpoint_record")]
        pub struct Model {
            #[sea_orm(primary_key, auto_increment = false)]
            pub id: Uuid,
            pub tenant_id: String,
            pub session_id: Uuid,
            pub run_id: Uuid,
            pub node: String,
            pub status: String,
            pub reason: String,
            pub parent_checkpoint_id: Option<Uuid>,
            pub metadata: Option<Value>,
            pub created_at: DateTime<Utc>,
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
                    "run_id" => Ok(CursorValue::Uuid(Some(self.run_id))),
                    "node" => Ok(CursorValue::String(Some(self.node.clone()))),
                    "status" => Ok(CursorValue::String(Some(self.status.clone()))),
                    "reason" => Ok(CursorValue::String(Some(self.reason.clone()))),
                    "parent_checkpoint_id" => Ok(CursorValue::Uuid(self.parent_checkpoint_id)),
                    "created_at" => Ok(CursorValue::DateTime(Some(self.created_at))),
                    _ => Err(DatabaseError::UnknownFieldName(field_name.to_string())),
                }
            }
        }

        impl From<Model> for CheckpointRecord {
            fn from(model: Model) -> Self {
                CheckpointRecord {
                    id: model.id,
                    tenant_id: model.tenant_id,
                    session_id: model.session_id,
                    run_id: model.run_id,
                    node: model.node,
                    status: RunStatus::from_str(&model.status).unwrap_or(RunStatus::Failed),
                    reason: CheckpointReason::from_str(&model.reason)
                        .unwrap_or(CheckpointReason::Step),
                    parent_checkpoint_id: model.parent_checkpoint_id,
                    metadata: model.metadata,
                    created_at: model.created_at,
                }
            }
        }
    }
}

pub struct SqlCheckpointRecordRepository<'a> {
    ctx: ConnectionContext<'a>,
}

impl<'a> SqlCheckpointRecordRepository<'a> {
    pub fn new(ctx: ConnectionContext<'a>) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl<'a> CheckpointRecordRepository for SqlCheckpointRecordRepository<'a> {
    async fn save(
        &self,
        records: Vec<CheckpointRecord>,
    ) -> Result<Vec<CheckpointRecord>, CheckpointRecordRepoError> {
        if records.is_empty() {
            return Ok(vec![]);
        }

        models::checkpoint_record::Entity::insert_many(records.into_iter().map(|r| {
            models::checkpoint_record::ActiveModel {
                id: ActiveValue::set(r.id),
                tenant_id: ActiveValue::set(r.tenant_id),
                session_id: ActiveValue::set(r.session_id),
                run_id: ActiveValue::set(r.run_id),
                node: ActiveValue::set(r.node),
                status: ActiveValue::set(r.status.to_string()),
                reason: ActiveValue::set(r.reason.to_string()),
                parent_checkpoint_id: ActiveValue::set(r.parent_checkpoint_id),
                metadata: ActiveValue::set(r.metadata),
                created_at: ActiveValue::set(r.created_at),
            }
        }))
        .on_conflict(
            OnConflict::columns([
                models::checkpoint_record::Column::Id,
                models::checkpoint_record::Column::TenantId,
            ])
            .update_columns([
                models::checkpoint_record::Column::Status,
                models::checkpoint_record::Column::Metadata,
            ])
            .to_owned(),
        )
        .exec_with_returning_many(&self.ctx)
        .await
        .map_err(CheckpointRecordRepoError::sourced_storage)
        .map(|models| {
            models
                .into_iter()
                .map(Into::into)
                .collect()
        })
    }

    async fn get(
        &self,
        tenant_id: &str,
        id: Uuid,
    ) -> Result<Option<CheckpointRecord>, CheckpointRecordRepoError> {
        models::checkpoint_record::Entity::find()
            .filter(models::checkpoint_record::Column::TenantId.eq(tenant_id.to_string()))
            .filter(models::checkpoint_record::Column::Id.eq(id))
            .one(&self.ctx)
            .await
            .map_err(CheckpointRecordRepoError::sourced_storage)
            .map(|opt| opt.map(Into::into))
    }

    async fn find(
        &self,
        params: FindCheckpointRecordParams,
    ) -> Result<Page<CheckpointRecord>, CheckpointRecordRepoError> {
        models::checkpoint_record::Entity::find()
            .apply_if(params.tenant_ids, |query, val| {
                query.filter(models::checkpoint_record::Column::TenantId.is_in(val))
            })
            .apply_if(params.ids, |query, val| {
                query.filter(models::checkpoint_record::Column::Id.is_in(val))
            })
            .apply_if(params.session_ids, |query, val| {
                query.filter(models::checkpoint_record::Column::SessionId.is_in(val))
            })
            .apply_if(params.run_ids, |query, val| {
                query.filter(models::checkpoint_record::Column::RunId.is_in(val))
            })
            .apply_if(params.reasons, |query, val| {
                query.filter(
                    models::checkpoint_record::Column::Reason.is_in(
                        val.into_iter()
                            .map(|r| r.to_string())
                            .collect::<Vec<_>>(),
                    ),
                )
            })
            .apply_if(params.statuses, |query, val| {
                query.filter(
                    models::checkpoint_record::Column::Status.is_in(
                        val.into_iter()
                            .map(|s| s.to_string())
                            .collect::<Vec<_>>(),
                    ),
                )
            })
            .apply_if(params.created_before, |query, val| {
                query.filter(models::checkpoint_record::Column::CreatedAt.lte(val))
            })
            .apply_if(params.created_after, |query, val| {
                query.filter(models::checkpoint_record::Column::CreatedAt.gte(val))
            })
            .cursor_paginate()
            .per_page(params.per_page)
            .cursor(params.page)
            .sort_desc(models::checkpoint_record::Column::CreatedAt.to_string())
            .execute(&self.ctx)
            .await
            .map_err(CheckpointRecordRepoError::sourced_storage)
            .map(|page| Page {
                items: page
                    .items
                    .into_iter()
                    .map(Into::into)
                    .collect(),
                count: page.count,
                next_page: page.next_page,
            })
    }

    async fn delete(
        &self,
        params: DeleteCheckpointRecordParams,
    ) -> Result<(), CheckpointRecordRepoError> {
        models::checkpoint_record::Entity::delete_many()
            .filter(models::checkpoint_record::Column::TenantId.eq(params.tenant_id))
            .filter(models::checkpoint_record::Column::Id.is_in(params.ids))
            .exec(&self.ctx)
            .await
            .map_err(CheckpointRecordRepoError::sourced_storage)
            .map(|_| ())
    }
}
