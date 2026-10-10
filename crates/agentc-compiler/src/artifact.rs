// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::path::PathBuf;

/// An executable file produced by a compiler.
pub struct ExecutableArtifact {
    pub path: PathBuf,
}

impl ExecutableArtifact {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
}
