// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::fmt::{Display, Formatter, Result as Fmtresult};

use crate::types::identity::{ModelId, StaticProviderId};

pub const PROVIDER: StaticProviderId = StaticProviderId::new("anthropic");

pub enum Model {
    ClaudeOpus46,
    ClaudeSonnet46,
    ClaudeHaiku45,
    ClaudeHaiku45_20251001,
}

impl Display for Model {
    fn fmt(&self, f: &mut Formatter<'_>) -> Fmtresult {
        match self {
            Self::ClaudeOpus46 => write!(f, "claude-opus-4-6"),
            Self::ClaudeSonnet46 => write!(f, "claude-sonnet-4-6"),
            Self::ClaudeHaiku45 => write!(f, "claude-haiku-4-5"),
            Self::ClaudeHaiku45_20251001 => write!(f, "claude-haiku-4-5-20251001"),
        }
    }
}

impl From<Model> for ModelId {
    fn from(value: Model) -> Self {
        ModelId::from(value.to_string())
    }
}

impl From<Model> for String {
    fn from(value: Model) -> Self {
        value.to_string()
    }
}
