// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use thiserror::Error;
use uuid::Uuid;

use agentc_agent::errors::AgentError;
use agentc_domain::repository::{
    checkpoint_record::errors::CheckpointRecordRepoError, run::errors::RunRepoError,
    session::errors::SessionRepoError,
};
use agentc_domain_sql::scope::SqlScopeFactoryError;
use agentc_http::errors::ApiError;

use crate::repository::message::errors::MessageRepoError;

#[derive(Error, Debug)]
pub enum ServiceError {
    #[error("session not found: {0}")]
    SessionNotFound(Uuid),

    #[error("session already exists: {0}")]
    SessionAlreadyExists(Uuid),

    #[error("run not found: {0}")]
    RunNotFound(Uuid),

    #[error("run already exists: {0}")]
    RunAlreadyExists(Uuid),

    #[error("message not found: {0}")]
    MessageNotFound(Uuid),

    #[error("message already exists: {0}")]
    MessageAlreadyExists(Uuid),

    #[error("session repo error: {0}")]
    SessionRepo(#[from] SessionRepoError),

    #[error("run repo error: {0}")]
    RunRepo(#[from] RunRepoError),

    #[error("message repo error: {0}")]
    MessageRepo(#[from] MessageRepoError),

    #[error("checkpoint record repo error: {0}")]
    CheckpointRecordRepo(#[from] CheckpointRecordRepoError),

    #[error("scope error: {0}")]
    Scope(#[from] SqlScopeFactoryError),

    #[error("agent error: {0}")]
    Agent(#[from] AgentError),

    #[error("unexpected error: {message}")]
    Unexpected {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
}

impl ServiceError {
    pub fn session_not_found(id: impl Into<Uuid>) -> Self {
        ServiceError::SessionNotFound(id.into())
    }

    pub fn session_already_exists(id: impl Into<Uuid>) -> Self {
        ServiceError::SessionAlreadyExists(id.into())
    }

    pub fn run_not_found(id: impl Into<Uuid>) -> Self {
        ServiceError::RunNotFound(id.into())
    }

    pub fn run_already_exists(id: impl Into<Uuid>) -> Self {
        ServiceError::RunAlreadyExists(id.into())
    }

    pub fn message_not_found(id: impl Into<Uuid>) -> Self {
        ServiceError::MessageNotFound(id.into())
    }

    pub fn message_already_exists(id: impl Into<Uuid>) -> Self {
        ServiceError::MessageAlreadyExists(id.into())
    }

    pub fn unexpected(message: impl Into<String>) -> Self {
        ServiceError::Unexpected { message: message.into(), source: None }
    }

    pub fn sourced_unexpected(
        message: impl Into<String>,
        source: impl Into<Box<dyn std::error::Error + Send + Sync>>,
    ) -> Self {
        ServiceError::Unexpected {
            message: message.into(),
            source: Some(source.into()),
        }
    }
}

impl From<ServiceError> for ApiError {
    fn from(err: ServiceError) -> Self {
        match err {
            ServiceError::SessionNotFound(_) => ApiError::new(404010, err.to_string()),
            ServiceError::SessionAlreadyExists(_) => ApiError::new(400010, err.to_string()),
            ServiceError::RunNotFound(_) => ApiError::new(404011, err.to_string()),
            ServiceError::RunAlreadyExists(_) => ApiError::new(400011, err.to_string()),
            ServiceError::MessageNotFound(_) => ApiError::new(404012, err.to_string()),
            ServiceError::MessageAlreadyExists(_) => ApiError::new(400012, err.to_string()),
            _ => ApiError::unexpected_error(err.to_string()),
        }
    }
}
