// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;

use agentc_domain::types::Page;

use crate::{
    repository::state_snapshot::{
        errors::StateSnapshotRepoError,
        params::{DeleteStateSnapshotParams, FindStateSnapshotParams},
    },
    types::state_snapshot::StateSnapshot,
};

#[async_trait]
pub trait StateSnapshotRepository: Send + Sync {
    async fn save(
        &self,
        snapshots: Vec<StateSnapshot>,
    ) -> Result<Vec<StateSnapshot>, StateSnapshotRepoError>;
    async fn find(
        &self,
        params: FindStateSnapshotParams,
    ) -> Result<Page<StateSnapshot>, StateSnapshotRepoError>;
    async fn delete(&self, params: DeleteStateSnapshotParams)
    -> Result<(), StateSnapshotRepoError>;
}

#[async_trait]
pub trait StateSnapshotRepoProvider {
    type Repo<'a>: StateSnapshotRepository + Send
    where
        Self: 'a;

    fn state_snapshot_repo(&self) -> Self::Repo<'_>;
}
