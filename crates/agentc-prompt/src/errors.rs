// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use thiserror::Error;

#[derive(Debug, Error)]
pub enum PromptError {
    #[error("render error: {0}")]
    Render(String),

    #[error("context error: {0}")]
    Context(String),

    #[error("prompt source error: {message}")]
    Source {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
}

impl PromptError {
    pub fn render(message: impl Into<String>) -> Self {
        Self::Render(message.into())
    }

    pub fn context(message: impl Into<String>) -> Self {
        Self::Context(message.into())
    }

    pub fn source(message: impl Into<String>) -> Self {
        Self::Source { message: message.into(), source: None }
    }

    pub fn sourced_source(
        message: impl Into<String>,
        source: impl Into<Box<dyn std::error::Error + Send + Sync>>,
    ) -> Self {
        Self::Source {
            message: message.into(),
            source: Some(source.into()),
        }
    }
}
