// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use thiserror::Error;

#[derive(Debug, Error)]
pub enum TransformError {
    #[error("I/O error while transforming '{path}': {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("transform failed for '{path}': {message}")]
    Failed { path: String, message: String },

    #[error("required tool not found: '{tool}' - {message}")]
    ToolNotFound { tool: String, message: String },
}

impl TransformError {
    pub fn io(path: impl Into<String>, source: std::io::Error) -> Self {
        Self::Io { path: path.into(), source }
    }

    pub fn failed(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Failed {
            path: path.into(),
            message: message.into(),
        }
    }

    pub fn tool_not_found(tool: impl Into<String>, message: impl Into<String>) -> Self {
        Self::ToolNotFound {
            tool: tool.into(),
            message: message.into(),
        }
    }
}
