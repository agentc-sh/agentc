// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use uuid::Uuid;

use agentc_database::{
    connection::ConnectionContext,
    orm::{ColumnTrait, Condition, EntityTrait, Iden, QueryFilter, QueryTrait},
    paginate::CursorPaginatorExt,
    query::OnConflict,
};
use agentc_domain::types::Page;
use agentc_domain_sql::scope::SqlScope;

use crate::{
    repository::message::{
        errors::MessageRepoError,
        params::{DeleteMessageParams, FindMessageParams},
        sql::models,
        traits::{MessageRepoProvider, MessageRepository},
    },
    types::message::Message,
};

pub struct SqlMessageRepository<'a> {
    ctx: ConnectionContext<'a>,
}

impl<'a> SqlMessageRepository<'a> {
    pub fn new(ctx: ConnectionContext<'a>) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl<'a> MessageRepository for SqlMessageRepository<'a> {
    async fn save(&self, messages: Vec<Message>) -> Result<Vec<Message>, MessageRepoError> {
        if messages.is_empty() {
            return Ok(vec![]);
        }

        models::message::Entity::insert_many(
            messages
                .into_iter()
                .map(models::message::ActiveModel::try_from)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| MessageRepoError::unexpected(e.to_string()))?,
        )
        .on_conflict(
            OnConflict::columns([
                models::message::Column::Id,
                models::message::Column::TenantId,
            ])
            .do_nothing()
            .to_owned(),
        )
        .exec_with_returning_many(&self.ctx)
        .await
        .map_err(MessageRepoError::sourced_storage)
        .and_then(|models| {
            models
                .into_iter()
                .map(Message::try_from)
                .collect::<Result<Vec<_>, _>>()
                .map_err(MessageRepoError::unexpected)
        })
    }

    async fn get(&self, tenant_id: &str, id: Uuid) -> Result<Option<Message>, MessageRepoError> {
        models::message::Entity::find()
            .filter(models::message::Column::TenantId.eq(tenant_id))
            .filter(models::message::Column::Id.eq(id))
            .one(&self.ctx)
            .await
            .map_err(MessageRepoError::sourced_storage)
            .and_then(|opt| {
                opt.map(Message::try_from)
                    .transpose()
                    .map_err(MessageRepoError::unexpected)
            })
    }

    async fn find(&self, params: FindMessageParams) -> Result<Page<Message>, MessageRepoError> {
        models::message::Entity::find()
            .apply_if(params.tenant_ids, |query, value| {
                query.filter(models::message::Column::TenantId.is_in(value))
            })
            .apply_if(params.ids, |query, value| {
                query.filter(models::message::Column::Id.is_in(value))
            })
            .apply_if(params.session_ids, |query, value| {
                query.filter(models::message::Column::SessionId.is_in(value))
            })
            .apply_if(params.run_ids, |query, value| {
                query.filter(models::message::Column::RunId.is_in(value))
            })
            .apply_if(params.checkpoint_ids, |query, value| {
                query.filter(
                    Condition::any()
                        .add(models::message::Column::CheckpointId.is_in(value))
                        .add(models::message::Column::CheckpointId.is_null()),
                )
            })
            .apply_if(params.roles, |query, value| {
                query.filter(
                    models::message::Column::Role.is_in(
                        value
                            .into_iter()
                            .map(|r| r.to_string())
                            .collect::<Vec<_>>(),
                    ),
                )
            })
            .apply_if(params.created_before, |query, value| {
                query.filter(models::message::Column::CreatedAt.lte(value))
            })
            .apply_if(params.created_after, |query, value| {
                query.filter(models::message::Column::CreatedAt.gte(value))
            })
            .cursor_paginate()
            .per_page(params.per_page)
            .cursor(params.page)
            .sort_desc(models::message::Column::CreatedAt.to_string())
            .execute(&self.ctx)
            .await
            .map_err(MessageRepoError::sourced_storage)
            .and_then(|page| {
                Ok(Page {
                    items: page
                        .items
                        .into_iter()
                        .map(Message::try_from)
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(MessageRepoError::unexpected)?,
                    count: page.count,
                    next_page: page.next_page,
                })
            })
    }

    async fn delete(&self, params: DeleteMessageParams) -> Result<(), MessageRepoError> {
        models::message::Entity::delete_many()
            .filter(models::message::Column::TenantId.eq(params.tenant_id))
            .filter(models::message::Column::Id.is_in(params.ids))
            .exec(&self.ctx)
            .await
            .map_err(MessageRepoError::sourced_storage)?;

        Ok(())
    }
}

impl MessageRepoProvider for ConnectionContext<'_> {
    type Repo<'a>
        = SqlMessageRepository<'a>
    where
        Self: 'a;

    fn message_repo(&self) -> Self::Repo<'_> {
        SqlMessageRepository::new(*self)
    }
}

impl MessageRepoProvider for SqlScope<'_> {
    type Repo<'a>
        = SqlMessageRepository<'a>
    where
        Self: 'a;

    fn message_repo(&self) -> Self::Repo<'_> {
        SqlMessageRepository::new(self.ctx())
    }
}
