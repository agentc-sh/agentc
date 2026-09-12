// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

/// The terminal status of a command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CmdOutcome {
    Success,
    Failure { code: i32 },
}

impl CmdOutcome {
    pub fn failure(code: i32) -> Self {
        Self::Failure { code }
    }

    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success)
    }

    /// The status the process exits with.
    pub fn code(&self) -> i32 {
        match self {
            Self::Success => 0,
            Self::Failure { code } => *code,
        }
    }
}
