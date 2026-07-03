// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use thiserror::Error;

use agentc_agent::tools::errors::ToolError;

#[derive(Debug, Error)]
pub enum BashToolError {
    /// The interpreter returned an error during script execution.
    #[error("bash execution error: {0}")]
    Execution(rust_bash::RustBashError),

    /// A filesystem backend could not be initialized.
    #[error("bash filesystem error: {0}")]
    Fs(std::io::Error),

    /// The blocking task was cancelled or panicked.
    #[error("bash worker thread panicked")]
    WorkerPanicked,
}

impl BashToolError {
    pub fn execution(err: rust_bash::RustBashError) -> Self {
        Self::Execution(err)
    }

    pub fn fs(err: std::io::Error) -> Self {
        Self::Fs(err)
    }

    pub fn worker_panicked() -> Self {
        Self::WorkerPanicked
    }
}

impl From<BashToolError> for ToolError {
    fn from(err: BashToolError) -> Self {
        ToolError::sourced_execution_error("bash", err.to_string(), Some(err))
    }
}
