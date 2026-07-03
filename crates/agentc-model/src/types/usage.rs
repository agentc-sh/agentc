// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};

/// Token usage reported at the end of a completion stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_input_tokens: Option<u32>,
}

impl TokenUsage {
    /// Total tokens used (input + output).
    pub fn total_tokens(&self) -> u32 {
        self.input_tokens + self.output_tokens
    }
}
