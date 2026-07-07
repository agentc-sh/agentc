// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

use agentc_agent::graph::cancel::{CancellationError, Canceller};
use agentc_database::database::Database;
use agentc_domain::{
    repository::run::{
        params::{Comparison, UpdateRunParams, UpdateRunParamsCondition, UpdateRunParamsSet},
        traits::RunRepository,
    },
    types::RunStatus as DomainRunStatus,
};
use agentc_domain_sql::repository::run::SqlRunRepository;

/// A [`Canceller`](agentc_agent::graph::cancel::Canceller) backed by the run
/// repository, shared with [`SqlReActSessionStore`](crate::checkpoint::store::SqlReActSessionStore).
pub struct SqlReActCanceller {
    database: Arc<Database>,
}

impl SqlReActCanceller {
    pub fn new(database: Arc<Database>) -> Self {
        Self { database }
    }
}

#[async_trait]
impl Canceller for SqlReActCanceller {
    async fn cancel(&self, tenant_id: &str, run_id: Uuid) -> Result<bool, CancellationError> {
        SqlRunRepository::new(self.database.write_ctx())
            .update(
                UpdateRunParams::new(tenant_id, run_id)
                    .set(UpdateRunParamsSet::Status(DomainRunStatus::Cancelled))
                    .condition(UpdateRunParamsCondition::Status(Comparison::Eq(
                        DomainRunStatus::Running,
                    ))),
            )
            .await
            .map_err(|e| CancellationError::store_error(e.to_string()))
    }

    async fn is_cancelled(&self, tenant_id: &str, run_id: Uuid) -> Result<bool, CancellationError> {
        Ok(SqlRunRepository::new(self.database.read_ctx())
            .get(tenant_id, run_id)
            .await
            .map_err(|e| CancellationError::store_error(e.to_string()))?
            .map(|run| matches!(run.status, DomainRunStatus::Cancelled))
            .unwrap_or(false))
    }
}
