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
    repository::session::{
        errors::SessionRepoError,
        params::{DeleteSessionParams, FindSessionParams},
        traits::SessionRepository,
    },
    types::{Page, Session},
};

pub mod models {
    pub mod session {
        use agentc_database::{
            errors::DatabaseError,
            orm::prelude::*,
            paginate::{CursorValue, ExtractCursorValue},
        };
        use async_trait::async_trait;
        use chrono::{DateTime, Utc};
        use uuid::Uuid;

        #[derive(Debug, Clone, PartialEq, Eq, DeriveEntityModel)]
        #[sea_orm(table_name = "session")]
        pub struct Model {
            #[sea_orm(primary_key, auto_increment = false)]
            pub id: Uuid,
            pub tenant_id: String,
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
                    "created_at" => Ok(CursorValue::DateTime(Some(self.created_at))),
                    "updated_at" => Ok(CursorValue::DateTime(Some(self.updated_at))),
                    _ => Err(DatabaseError::UnknownFieldName(field_name.to_string())),
                }
            }
        }
    }
}

pub struct SqlSessionRepository<'a> {
    ctx: ConnectionContext<'a>,
}

impl<'a> SqlSessionRepository<'a> {
    pub fn new(ctx: ConnectionContext<'a>) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl<'a> SessionRepository for SqlSessionRepository<'a> {
    async fn save(&self, sessions: Vec<Session>) -> Result<Vec<Session>, SessionRepoError> {
        if sessions.is_empty() {
            return Ok(vec![]);
        }

        models::session::Entity::insert_many(sessions.into_iter().map(|session| {
            models::session::ActiveModel {
                id: ActiveValue::set(session.id),
                tenant_id: ActiveValue::set(session.tenant_id),
                created_at: ActiveValue::set(session.created_at),
                updated_at: ActiveValue::set(session.updated_at),
            }
        }))
        .on_conflict(
            OnConflict::columns([
                models::session::Column::Id,
                models::session::Column::TenantId,
            ])
            .update_column(models::session::Column::UpdatedAt)
            .to_owned(),
        )
        .exec_with_returning_many(&self.ctx)
        .await
        .map_err(SessionRepoError::sourced_storage)
        .map(|models| {
            models
                .into_iter()
                .map(|model| Session {
                    id: model.id,
                    tenant_id: model.tenant_id,
                    created_at: model.created_at,
                    updated_at: model.updated_at,
                })
                .collect()
        })
    }

    async fn get(&self, tenant_id: &str, id: Uuid) -> Result<Option<Session>, SessionRepoError> {
        models::session::Entity::find()
            .filter(models::session::Column::TenantId.eq(tenant_id))
            .filter(models::session::Column::Id.eq(id))
            .one(&self.ctx)
            .await
            .map_err(SessionRepoError::sourced_storage)
            .map(|opt| {
                opt.map(|model| Session {
                    id: model.id,
                    tenant_id: model.tenant_id,
                    created_at: model.created_at,
                    updated_at: model.updated_at,
                })
            })
    }

    async fn find(&self, params: FindSessionParams) -> Result<Page<Session>, SessionRepoError> {
        models::session::Entity::find()
            .apply_if(params.tenant_ids, |query, val| {
                query.filter(models::session::Column::TenantId.is_in(val))
            })
            .apply_if(params.ids, |query, val| query.filter(models::session::Column::Id.is_in(val)))
            .apply_if(params.created_before, |query, val| {
                query.filter(models::session::Column::CreatedAt.lte(val))
            })
            .apply_if(params.created_after, |query, val| {
                query.filter(models::session::Column::CreatedAt.gte(val))
            })
            .apply_if(params.updated_before, |query, val| {
                query.filter(models::session::Column::UpdatedAt.lte(val))
            })
            .apply_if(params.updated_after, |query, val| {
                query.filter(models::session::Column::UpdatedAt.gte(val))
            })
            .cursor_paginate()
            .per_page(params.per_page)
            .cursor(params.page)
            .sort_desc(models::session::Column::CreatedAt.to_string())
            .execute(&self.ctx)
            .await
            .map_err(SessionRepoError::sourced_storage)
            .map(|page| Page {
                items: page
                    .items
                    .into_iter()
                    .map(|model| Session {
                        id: model.id,
                        tenant_id: model.tenant_id,
                        created_at: model.created_at,
                        updated_at: model.updated_at,
                    })
                    .collect(),
                count: page.count,
                next_page: page.next_page,
            })
    }

    async fn delete(&self, params: DeleteSessionParams) -> Result<(), SessionRepoError> {
        models::session::Entity::delete_many()
            .filter(models::session::Column::TenantId.eq(params.tenant_id))
            .filter(models::session::Column::Id.is_in(params.ids))
            .exec(&self.ctx)
            .await
            .map_err(SessionRepoError::sourced_storage)
            .map(|_| ())
    }
}
