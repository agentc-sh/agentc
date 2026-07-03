// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use thiserror::Error;

#[derive(Debug, Error)]
pub enum MessageRepoError {
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

impl MessageRepoError {
    pub fn sourced_storage(source: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> Self {
        let source = source.into();
        Self::Storage {
            message: source.to_string(),
            source: Some(source),
        }
    }

    pub fn unexpected(message: impl Into<String>) -> Self {
        Self::Unexpected { message: message.into(), source: None }
    }
}
