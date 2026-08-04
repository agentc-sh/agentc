// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use thiserror::Error;

use crate::{compiler::errors::CompilerError, runner::errors::RunnerError};

#[derive(Error, Debug)]
pub enum ToolchainError {
    #[error("compiler error: {0}")]
    Compiler(#[from] CompilerError),

    #[error("runner error: {0}")]
    Runner(#[from] RunnerError),

    #[error("this toolchain cannot invoke what it builds")]
    RunUnsupported,

    #[error("nothing has been built yet")]
    NotBuilt,
}
