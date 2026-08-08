// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;

use crate::runner::{
    errors::RunnerError,
    types::{RunOutcome, RunParams},
};

#[async_trait]
pub trait Runner: Send + Sync {
    /// The artifact this runner invokes.
    type Artifact: Send + Sync + 'static;

    async fn run(
        &self,
        artifact: &Self::Artifact,
        params: RunParams,
    ) -> Result<RunOutcome, RunnerError>;
}
