// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use thiserror::Error;

#[derive(Error, Debug)]
pub enum CompilerError {
    #[error("Compilation failed: {message}")]
    CompilationFailed {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("Unsupported target platform: {0}")]
    UnsupportedTargetPlatform(String),

    #[error("Missing build configuration")]
    MissingBuildConfiguration,

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

impl CompilerError {
    pub fn compilation_failed_sourced(
        message: impl Into<String>,
        source: Option<impl Into<Box<dyn std::error::Error + Send + Sync>>>,
    ) -> Self {
        Self::CompilationFailed {
            message: message.into(),
            source: source.map(Into::into),
        }
    }

    pub fn compilation_failed(message: impl Into<String>) -> Self {
        Self::CompilationFailed { message: message.into(), source: None }
    }
}
