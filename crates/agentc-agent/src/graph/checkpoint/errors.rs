// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CheckpointError {
    #[error("session store error: {0}")]
    SessionStore(String),

    #[error("checkpoint store error: {0}")]
    CheckpointStore(String),

    #[error("state store error: {0}")]
    StateStore(String),

    #[error("unexpected error: {0}")]
    Unexpected(String),
}

impl CheckpointError {
    pub fn session_store_error(msg: impl Into<String>) -> Self {
        Self::SessionStore(msg.into())
    }

    pub fn checkpoint_store_error(msg: impl Into<String>) -> Self {
        Self::CheckpointStore(msg.into())
    }

    pub fn state_store_error(msg: impl Into<String>) -> Self {
        Self::StateStore(msg.into())
    }

    pub fn unexpected_error(msg: impl Into<String>) -> Self {
        Self::Unexpected(msg.into())
    }
}
