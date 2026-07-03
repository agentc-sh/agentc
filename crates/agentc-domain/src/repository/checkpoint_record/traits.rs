// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    repository::checkpoint_record::{
        errors::CheckpointRecordRepoError,
        params::{DeleteCheckpointRecordParams, FindCheckpointRecordParams},
    },
    types::{CheckpointRecord, Page},
};

#[async_trait]
pub trait CheckpointRecordRepository: Send + Sync {
    async fn save(
        &self,
        records: Vec<CheckpointRecord>,
    ) -> Result<Vec<CheckpointRecord>, CheckpointRecordRepoError>;
    async fn get(
        &self,
        tenant_id: &str,
        id: Uuid,
    ) -> Result<Option<CheckpointRecord>, CheckpointRecordRepoError>;
    async fn find(
        &self,
        params: FindCheckpointRecordParams,
    ) -> Result<Page<CheckpointRecord>, CheckpointRecordRepoError>;
    async fn delete(
        &self,
        params: DeleteCheckpointRecordParams,
    ) -> Result<(), CheckpointRecordRepoError>;
}

#[async_trait]
pub trait CheckpointRecordRepoProvider {
    type Repo<'a>: CheckpointRecordRepository + Send
    where
        Self: 'a;

    fn checkpoint_record_repo(&self) -> Self::Repo<'_>;
}
