// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};
use serde_json::to_string_pretty;
use std::fmt::{Display, Formatter, Result as FmtResult};
use thiserror::Error;

#[derive(Error, Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum CliError {
    Generic {
        code: i32,
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        hint: Option<String>,
    },
    Validation {
        code: i32,
        errors: validator::ValidationErrors,
    },
}

impl CliError {
    pub fn code(&self) -> i32 {
        match self {
            Self::Generic { code, .. } => *code,
            Self::Validation { code, .. } => *code,
        }
    }

    pub fn exit(self) -> ! {
        println!("{}", self);
        std::process::exit(self.code());
    }

    pub fn unexpected_error(message: impl Into<String>) -> Self {
        Self::Generic {
            code: 1,
            message: message.into(),
            hint: None,
        }
    }

    pub fn invalid_parameters(message: impl Into<String>) -> Self {
        Self::Generic {
            code: 2,
            message: message.into(),
            hint: None,
        }
    }

    pub fn validation_error(errors: validator::ValidationErrors) -> Self {
        Self::Validation { code: 3, errors }
    }

    pub fn io_error(message: impl Into<String>) -> Self {
        Self::Generic {
            code: 4,
            message: message.into(),
            hint: None,
        }
    }
}

impl Display for CliError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", to_string_pretty(&self).expect("Failed to serialize error"))
    }
}

impl From<anyhow::Error> for CliError {
    fn from(error: anyhow::Error) -> Self {
        CliError::unexpected_error(error.to_string())
    }
}

impl From<std::io::Error> for CliError {
    fn from(error: std::io::Error) -> Self {
        CliError::io_error(error.to_string())
    }
}

impl From<validator::ValidationErrors> for CliError {
    fn from(errors: validator::ValidationErrors) -> Self {
        CliError::validation_error(errors)
    }
}
