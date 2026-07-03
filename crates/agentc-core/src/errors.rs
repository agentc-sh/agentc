// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use thiserror::Error;

use crate::{build::errors::BuildError, parser::errors::ParserError};

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("build error: {0}")]
    Build(#[from] BuildError),

    #[error("parser error: {0}")]
    Parser(#[from] ParserError),
}
