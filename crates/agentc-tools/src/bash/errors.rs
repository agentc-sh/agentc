// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use agentc_agent::tools::errors::ToolError;
use bashkit::Error as BashkitError;

#[derive(Debug, thiserror::Error)]
pub enum BashToolError {
    #[error("bash execution error: {0}")]
    Execution(BashkitError),
}

impl BashToolError {
    pub fn execution(error: BashkitError) -> Self {
        BashToolError::Execution(error)
    }
}

impl From<BashToolError> for ToolError {
    fn from(error: BashToolError) -> Self {
        ToolError::sourced_execution_error("bash", error.to_string(), Some(error))
    }
}
