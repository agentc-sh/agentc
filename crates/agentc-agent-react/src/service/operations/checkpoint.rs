// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;

use agentc_domain::{
    repository::{
        checkpoint_record::traits::{CheckpointRecordRepoProvider, CheckpointRecordRepository},
        scope::RepoScopeFactory,
    },
    types::Page,
};
use agentc_telemetry::{Level, instrument};

use crate::service::{
    application::ApplicationService,
    errors::ServiceError,
    types::checkpoint::{CheckpointResponse, FindCheckpointParams},
};

#[async_trait]
pub trait CheckpointOperations: Send + Sync {
    async fn find_checkpoints(
        &self,
        params: FindCheckpointParams,
    ) -> Result<Page<CheckpointResponse>, ServiceError>;
}

#[async_trait]
impl CheckpointOperations for ApplicationService {
    #[instrument(
        level = Level::TRACE,
        skip(self, params),
        fields(
            per_page = &params.per_page,
            page = &params.page,
            tenant_ids = ?params.tenant_ids,
            session_ids = ?params.session_ids,
        )
    )]
    async fn find_checkpoints(
        &self,
        params: FindCheckpointParams,
    ) -> Result<Page<CheckpointResponse>, ServiceError> {
        self.scope_factory
            .ro_scope(|scope| {
                Box::pin(async move {
                    Ok(scope
                        .checkpoint_record_repo()
                        .find(params.into())
                        .await?
                        .map(|record| CheckpointResponse::from_entity(&record)))
                })
            })
            .await
    }
}
