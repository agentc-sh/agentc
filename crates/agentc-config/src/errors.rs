// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use thiserror::Error;

use crate::path::Path;

#[derive(Debug, Error)]
pub enum SourceError {
    #[error("env var `{name}` not found")]
    EnvVarNotFound { name: String },

    #[error("{0}")]
    Custom(String),
}

impl SourceError {
    pub fn env_var_not_found(name: impl Into<String>) -> Self {
        Self::EnvVarNotFound { name: name.into() }
    }

    pub fn custom(msg: impl Into<String>) -> Self {
        Self::Custom(msg.into())
    }
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("missing required value at {path}")]
    MissingRequired { path: Path },

    #[error("parse failure at {path}: cannot parse `{raw}` as {target_type}: {source}")]
    ParseFailure {
        path: Path,
        raw: String,
        target_type: &'static str,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("type mismatch at {path}: expected {expected}, found {found}")]
    TypeMismatch {
        path: Path,
        expected: &'static str,
        found: &'static str,
    },

    #[error("merge conflict at {path}: cannot merge {incoming} into {existing}")]
    MergeConflict {
        path: Path,
        existing: &'static str,
        incoming: &'static str,
    },

    #[error("source `{name}` failed: {inner}")]
    Source { name: String, inner: SourceError },
}

impl ConfigError {
    pub fn missing_required(path: impl Into<Path>) -> Self {
        Self::MissingRequired { path: path.into() }
    }

    pub fn parse_failure(
        path: impl Into<Path>,
        raw: impl Into<String>,
        target_type: &'static str,
        source: impl Into<Box<dyn std::error::Error + Send + Sync>>,
    ) -> Self {
        Self::ParseFailure {
            path: path.into(),
            raw: raw.into(),
            target_type,
            source: source.into(),
        }
    }

    pub fn type_mismatch(
        path: impl Into<Path>,
        expected: &'static str,
        found: &'static str,
    ) -> Self {
        Self::TypeMismatch { path: path.into(), expected, found }
    }

    pub fn merge_conflict(
        path: impl Into<Path>,
        existing: &'static str,
        incoming: &'static str,
    ) -> Self {
        Self::MergeConflict { path: path.into(), existing, incoming }
    }

    pub fn source(name: impl Into<String>, inner: SourceError) -> Self {
        Self::Source { name: name.into(), inner }
    }
}

impl serde::de::Error for ConfigError {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        Self::ParseFailure {
            path: Path::default(),
            raw: String::new(),
            target_type: "unknown",
            source: msg.to_string().into(),
        }
    }
}
