// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use thiserror::Error;

#[derive(Debug, Error)]
pub enum PromptError {
    #[error("render error: {0}")]
    Render(String),

    #[error("context error: {0}")]
    Context(String),
}
