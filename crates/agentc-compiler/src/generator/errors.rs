// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::error::Error as StdError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GeneratorError {
    #[error("context serialization failed: {0}")]
    ContextSerialization(String),

    #[error(
        "block '{block_id}' strictly contributes to extension point '{point}' but no block declares it"
    )]
    UndeclaredExtensionPoint { block_id: String, point: String },

    #[error("block '{0}' is registered more than once")]
    DuplicateBlock(String),

    #[error("extension point '{point}' is declared more than once")]
    DuplicateExtensionPoint { point: String },

    #[error(
        "block '{block_id}' contributes to extension point '{point}' with type '{actual}', but the extension point expects '{expected}'"
    )]
    ExtensionPointTypeMismatch {
        block_id: String,
        point: String,
        expected: &'static str,
        actual: &'static str,
    },

    #[error("template '{template}' not found in block '{block_id}'")]
    TemplateNotFound { block_id: String, template: String },

    #[error("template render failed in block '{block_id}': {source}")]
    RenderFailed {
        block_id: String,
        #[source]
        source: minijinja::Error,
    },

    #[error("condition parse failed in block '{block_id}': {message}")]
    ConditionParseFailed { block_id: String, message: String },

    #[error("condition eval failed in block '{block_id}': {source}")]
    ConditionEvalFailed {
        block_id: String,
        #[source]
        source: cel::ExecutionError,
    },

    #[error("condition in block '{block_id}' did not evaluate to a boolean")]
    ConditionNotBoolean { block_id: String },

    #[error("resource not found: {0}")]
    ResourceNotFound(String),

    #[error("failed to load resource '{path}': {reason}")]
    ResourceLoadFailed { path: String, reason: String },

    #[error("unexpected error: {message}")]
    Unexpected {
        message: String,
        #[source]
        source: Option<Box<dyn StdError + Send + Sync>>,
    },
}

impl GeneratorError {
    pub fn resource_not_found(path: impl Into<String>) -> Self {
        Self::ResourceNotFound(path.into())
    }

    pub fn resource_load_failed(path: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::ResourceLoadFailed { path: path.into(), reason: reason.into() }
    }

    pub fn unexpected(message: impl Into<String>) -> Self {
        Self::Unexpected { message: message.into(), source: None }
    }

    pub fn sourced_unexpected(
        message: impl Into<String>,
        source: Option<impl Into<Box<dyn StdError + Send + Sync>>>,
    ) -> Self {
        Self::Unexpected {
            message: message.into(),
            source: source.map(|s| s.into()),
        }
    }
}
