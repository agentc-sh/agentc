// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::path::PathBuf;

/// Inputs for a single invocation of a built artifact.
#[derive(Debug, Clone)]
pub struct RunParams {
    /// The directory the manifest was read from.
    pub context_dir: PathBuf,
    /// Arguments forwarded to the invocation verbatim.
    pub args: Vec<String>,
}

impl RunParams {
    pub fn new(context_dir: impl Into<PathBuf>) -> Self {
        Self {
            context_dir: context_dir.into(),
            args: Vec::new(),
        }
    }

    pub fn with_args<I, A>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = A>,
        A: Into<String>,
    {
        self.args
            .extend(args.into_iter().map(Into::into));
        self
    }
}

/// The result of an invocation that ran to completion.
#[derive(Debug, Clone)]
pub struct RunOutcome {
    /// The status the invocation reported, if any.
    pub exit_code: Option<i32>,
}

impl RunOutcome {
    pub fn new(exit_code: Option<i32>) -> Self {
        Self { exit_code }
    }

    pub fn is_success(&self) -> bool {
        matches!(self.exit_code, Some(0))
    }
}
