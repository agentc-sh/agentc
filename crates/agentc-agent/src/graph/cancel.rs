// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum CancellationError {
    #[error("cancellation store error: {0}")]
    Store(String),

    #[error("unexpected cancellation error: {0}")]
    Unexpected(String),
}

impl CancellationError {
    pub fn store_error(message: impl Into<String>) -> Self {
        Self::Store(message.into())
    }

    pub fn unexpected_error(message: impl Into<String>) -> Self {
        Self::Unexpected(message.into())
    }
}

/// Handles run cancellation: causing a run to become cancelled, and observing
/// whether a run has been cancelled. Deliberately not generic over the graph
/// state type, because cancellation operates only on run identity and status.
#[async_trait]
pub trait Canceller: Send + Sync {
    /// Cause a run to be cancelled by transitioning it Running -> Cancelled.
    /// Conditional and idempotent: returns true if this call performed the
    /// transition, false if the run was already terminal or absent.
    async fn cancel(&self, tenant_id: &str, run_id: Uuid) -> Result<bool, CancellationError>;

    /// Whether the run is currently cancelled. Read by the run loop at safe points.
    async fn is_cancelled(&self, tenant_id: &str, run_id: Uuid) -> Result<bool, CancellationError>;
}
