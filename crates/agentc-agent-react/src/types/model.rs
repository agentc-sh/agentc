// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};

use agentc_model::types::inference::InferenceParams;

/// A request-time override for the model and/or inference parameters used
/// by the agent. All fields are optional -- set only the ones you want to
/// override.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelOverride {
    /// Override the provider (e.g. `"openai"`, `"anthropic"`).
    pub provider: Option<String>,
    /// Override the model name (e.g. `"gpt-4o"`, `"claude-sonnet-4-6"`).
    pub model: Option<String>,
    /// Override inference parameters. Fields set here take precedence over
    /// the agent identity defaults.
    pub inference_params: Option<InferenceParams>,
}
