// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use thiserror::Error;

/// An error produced by the Langfuse client.
#[derive(Debug, Error)]
pub enum LangfuseError {
    #[error("missing required Langfuse client field `{0}`")]
    MissingField(&'static str),

    #[error("invalid Langfuse client configuration: {0}")]
    Configuration(String),

    #[error("Langfuse request failed")]
    Request {
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("failed to decode Langfuse response")]
    Decode {
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("Langfuse API returned status {status}: {message}")]
    Response { status: u16, message: String },

    #[error("Langfuse prompt cache returned no entry")]
    Cache,
}

impl LangfuseError {
    pub(super) fn configuration(message: impl Into<String>) -> Self {
        Self::Configuration(message.into())
    }

    pub(super) fn request(source: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> Self {
        Self::Request { source: source.into() }
    }

    pub(super) fn decode(source: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> Self {
        Self::Decode { source: source.into() }
    }

    pub(super) fn response(status: u16, message: impl Into<String>) -> Self {
        Self::Response { status, message: message.into() }
    }
}
