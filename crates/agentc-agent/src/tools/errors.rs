// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ToolError {
    #[error("tool not found: {0}")]
    NotFound(String),

    #[error("invalid arguments: {0}")]
    InvalidArguments(String),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("execution error for tool '{name}': {message}")]
    ExecutionError {
        name: String,
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
}

impl ToolError {
    pub fn not_found(name: impl Into<String>) -> Self {
        Self::NotFound(name.into())
    }

    pub fn invalid_args(message: impl Into<String>) -> Self {
        Self::InvalidArguments(message.into())
    }

    pub fn sourced_execution_error<E>(
        name: impl Into<String>,
        message: impl Into<String>,
        source: Option<E>,
    ) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::ExecutionError {
            name: name.into(),
            message: message.into(),
            source: source.map(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>),
        }
    }

    pub fn execution_error(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self::ExecutionError {
            name: name.into(),
            message: message.into(),
            source: None,
        }
    }
}
