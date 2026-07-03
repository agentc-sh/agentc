// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use thiserror::Error;

use agentc_compiler::generator::errors::GeneratorError;

#[derive(Error, Debug)]
pub enum ManifestError {
    #[error("manifest resolution failed: {0}")]
    Resolution(String),

    #[error("generator error: {0}")]
    Generator(#[from] GeneratorError),
}

impl ManifestError {
    pub fn resolution(message: impl Into<String>) -> Self {
        Self::Resolution(message.into())
    }
}
