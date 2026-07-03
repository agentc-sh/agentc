// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use thiserror::Error;

#[derive(Debug, Error)]
pub enum SessionRepoError {
    #[error("storage error: {message}")]
    Storage {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("unexpected error: {message}")]
    Unexpected {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
}

impl SessionRepoError {
    /// Creates a new [`SessionRepoError::Storage`](crate::repository::session::errors::SessionRepoError::Storage)
    /// with the given message and source error.
    pub fn sourced_storage(source: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> Self {
        let source = source.into();
        Self::Storage {
            message: source.to_string(),
            source: Some(source),
        }
    }

    /// Creates a new [`SessionRepoError::Storage`](crate::repository::session::errors::SessionRepoError::Storage)
    /// with the given message and no source error.
    pub fn storage(message: impl Into<String>) -> Self {
        Self::Storage { message: message.into(), source: None }
    }

    /// Creates a new [`SessionRepoError::Unexpected`](crate::repository::session::errors::SessionRepoError::Unexpected)
    /// with the given message and source error.
    pub fn sourced_unexpected(source: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> Self {
        let source = source.into();
        Self::Unexpected {
            message: source.to_string(),
            source: Some(source),
        }
    }

    /// Creates a new [`SessionRepoError::Unexpected`](crate::repository::session::errors::SessionRepoError::Unexpected)
    /// with the given message and no source error.
    pub fn unexpected(message: impl Into<String>) -> Self {
        Self::Unexpected { message: message.into(), source: None }
    }
}
