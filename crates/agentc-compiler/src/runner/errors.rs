// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RunnerError {
    #[error("Invocation failed: {message}")]
    InvocationFailed {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("Nothing to invoke at {0}")]
    ArtifactMissing(PathBuf),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

impl RunnerError {
    pub fn invocation_failed_sourced(
        message: impl Into<String>,
        source: Option<impl Into<Box<dyn std::error::Error + Send + Sync>>>,
    ) -> Self {
        Self::InvocationFailed {
            message: message.into(),
            source: source.map(Into::into),
        }
    }

    pub fn invocation_failed(message: impl Into<String>) -> Self {
        Self::InvocationFailed { message: message.into(), source: None }
    }
}
