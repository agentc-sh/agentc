// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;

use agentc_database::{
    connection::ConnectionContext,
    orm::{ColumnTrait, EntityTrait, Iden, QueryFilter, QueryTrait},
    paginate::CursorPaginatorExt,
    query::OnConflict,
};
use agentc_domain::types::Page;
use agentc_domain_sql::scope::SqlScope;

use crate::{
    repository::state_snapshot::{
        errors::StateSnapshotRepoError,
        params::{DeleteStateSnapshotParams, FindStateSnapshotParams},
        sql::models,
        traits::{StateSnapshotRepoProvider, StateSnapshotRepository},
    },
    types::state_snapshot::StateSnapshot,
};

pub struct SqlStateSnapshotRepository<'a> {
    ctx: ConnectionContext<'a>,
}

impl<'a> SqlStateSnapshotRepository<'a> {
    pub fn new(ctx: ConnectionContext<'a>) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl<'a> StateSnapshotRepository for SqlStateSnapshotRepository<'a> {
    async fn save(
        &self,
        snapshots: Vec<StateSnapshot>,
    ) -> Result<Vec<StateSnapshot>, StateSnapshotRepoError> {
        if snapshots.is_empty() {
            return Ok(vec![]);
        }

        models::state_snapshot::Entity::insert_many(
            snapshots
                .into_iter()
                .map(models::state_snapshot::ActiveModel::try_from)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| StateSnapshotRepoError::unexpected(e.to_string()))?,
        )
        .on_conflict(
            OnConflict::columns([
                models::state_snapshot::Column::Id,
                models::state_snapshot::Column::TenantId,
            ])
            .update_columns([
                models::state_snapshot::Column::ModelOverride,
                models::state_snapshot::Column::CapabilityOverride,
                models::state_snapshot::Column::Tools,
                models::state_snapshot::Column::ContextVars,
                models::state_snapshot::Column::UpdatedAt,
            ])
            .to_owned(),
        )
        .exec_with_returning_many(&self.ctx)
        .await
        .map_err(StateSnapshotRepoError::sourced_storage)
        .and_then(|models| {
            models
                .into_iter()
                .map(StateSnapshot::try_from)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| StateSnapshotRepoError::unexpected(e.to_string()))
        })
    }

    async fn find(
        &self,
        params: FindStateSnapshotParams,
    ) -> Result<Page<StateSnapshot>, StateSnapshotRepoError> {
        models::state_snapshot::Entity::find()
            .apply_if(params.tenant_ids, |query, value| {
                query.filter(models::state_snapshot::Column::TenantId.is_in(value))
            })
            .apply_if(params.ids, |query, value| {
                query.filter(models::state_snapshot::Column::Id.is_in(value))
            })
            .apply_if(params.session_ids, |query, value| {
                query.filter(models::state_snapshot::Column::SessionId.is_in(value))
            })
            .apply_if(params.run_ids, |query, value| {
                query.filter(models::state_snapshot::Column::RunId.is_in(value))
            })
            .apply_if(params.checkpoint_ids, |query, value| {
                query.filter(models::state_snapshot::Column::CheckpointId.is_in(value))
            })
            .apply_if(params.created_before, |query, value| {
                query.filter(models::state_snapshot::Column::CreatedAt.lt(value))
            })
            .apply_if(params.created_after, |query, value| {
                query.filter(models::state_snapshot::Column::CreatedAt.gt(value))
            })
            .apply_if(params.updated_before, |query, value| {
                query.filter(models::state_snapshot::Column::UpdatedAt.lt(value))
            })
            .apply_if(params.updated_after, |query, value| {
                query.filter(models::state_snapshot::Column::UpdatedAt.gt(value))
            })
            .cursor_paginate()
            .per_page(params.per_page)
            .cursor(params.page)
            .sort_desc(models::state_snapshot::Column::CreatedAt.to_string())
            .execute(&self.ctx)
            .await
            .map_err(StateSnapshotRepoError::sourced_storage)
            .and_then(|page| {
                Ok(Page {
                    items: page
                        .items
                        .into_iter()
                        .map(StateSnapshot::try_from)
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|e| StateSnapshotRepoError::unexpected(e.to_string()))?,
                    count: page.count,
                    next_page: page.next_page,
                })
            })
    }

    async fn delete(
        &self,
        params: DeleteStateSnapshotParams,
    ) -> Result<(), StateSnapshotRepoError> {
        models::state_snapshot::Entity::delete_many()
            .filter(models::state_snapshot::Column::TenantId.eq(params.tenant_id))
            .filter(models::state_snapshot::Column::Id.is_in(params.ids))
            .exec(&self.ctx)
            .await
            .map_err(StateSnapshotRepoError::sourced_storage)?;

        Ok(())
    }
}

impl StateSnapshotRepoProvider for ConnectionContext<'_> {
    type Repo<'a>
        = SqlStateSnapshotRepository<'a>
    where
        Self: 'a;

    fn state_snapshot_repo(&self) -> Self::Repo<'_> {
        SqlStateSnapshotRepository::new(*self)
    }
}

impl StateSnapshotRepoProvider for SqlScope<'_> {
    type Repo<'a>
        = SqlStateSnapshotRepository<'a>
    where
        Self: 'a;

    fn state_snapshot_repo(&self) -> Self::Repo<'_> {
        SqlStateSnapshotRepository::new(self.ctx())
    }
}
