// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};

/// A reasoning block emitted by the model during extended thinking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reasoning {
    /// Provider-supplied identifier for this reasoning block, if present.
    pub id: Option<String>,
    /// The content of this reasoning block.
    pub content: Vec<ReasoningContent>,
}

/// The content of a reasoning block.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ReasoningContent {
    /// Plain reasoning text, with an optional provider signature for
    /// verification purposes.
    Text {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        signature: Option<String>,
    },
    /// An opaque provider-encrypted reasoning payload. Carried through
    /// as-is.
    Encrypted(String),
    /// A redacted reasoning payload preserved as opaque data.
    Redacted(String),
    /// A provider-generated summary of the reasoning.
    Summary(String),
}
