// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    repository::run::{
        errors::RunRepoError,
        params::{DeleteRunParams, FindRunParams},
    },
    types::{Page, Run},
};

#[async_trait]
pub trait RunRepository: Send + Sync {
    async fn save(&self, runs: Vec<Run>) -> Result<Vec<Run>, RunRepoError>;
    async fn get(&self, tenant_id: &str, id: Uuid) -> Result<Option<Run>, RunRepoError>;
    async fn find(&self, params: FindRunParams) -> Result<Page<Run>, RunRepoError>;
    async fn delete(&self, params: DeleteRunParams) -> Result<(), RunRepoError>;
}

#[async_trait]
pub trait RunRepoProvider {
    type Repo<'a>: RunRepository + Send
    where
        Self: 'a;

    fn run_repo(&self) -> Self::Repo<'_>;
}
