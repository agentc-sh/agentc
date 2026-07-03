// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use anyhow::Error as AnyhowError;
use std::error::Error as StdError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParserError {
    #[error("unexpected error: {message}")]
    Unexpected {
        message: String,
        #[source]
        source: Option<Box<dyn StdError + Send + Sync>>,
    },

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("failed to parse HCL: {0}")]
    Hcl(#[from] hcl::Error),

    #[error("failed to parse YAML: {0}")]
    Yml(#[from] serde_norway::Error),

    #[error("failed to parse JSON: {0}")]
    Json(#[from] serde_json::Error),

    #[error("parser error: {0}")]
    Parser(#[from] config::ConfigError),

    #[error("validation error: {0}")]
    Validation(#[from] validator::ValidationErrors),

    #[error("invalid expression: {0}")]
    InvalidExpression(String),

    #[error("unknown format: {0}")]
    UnknownFormat(String),
}

impl From<AnyhowError> for ParserError {
    fn from(err: AnyhowError) -> Self {
        ParserError::Unexpected {
            message: err.to_string(),
            source: Some(err.into()),
        }
    }
}
