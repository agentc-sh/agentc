// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::{
    error::Error,
    fmt::{Display, Formatter, Result as FmtResult},
};

use minijinja::Error as JinjaError;

#[derive(Debug)]
pub enum InitError {
    TemplateFailed { file: String, source: JinjaError },
}

impl Display for InitError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            InitError::TemplateFailed { file, source } => {
                write!(f, "template render failed for '{file}': {source}")
            }
        }
    }
}

impl Error for InitError {}
