// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};

use agentc_model::types::inference::InferenceParams;

/// Runtime model configuration used by a ReAct run.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelConfig {
    #[serde(rename = "override", skip_serializing_if = "Option::is_none")]
    pub r#override: Option<ModelConfigOverride>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry: Option<ModelConfigRetry>,
}

impl ModelConfig {
    pub fn new() -> Self {
        Self {
            r#override: None,
            timeout: None,
            retry: None,
        }
    }

    pub fn with_override(mut self, r#override: ModelConfigOverride) -> Self {
        self.r#override = Some(r#override);
        self
    }

    pub fn maybe_with_override(mut self, r#override: Option<ModelConfigOverride>) -> Self {
        self.r#override = r#override;
        self
    }

    pub fn with_timeout(mut self, timeout: u64) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn maybe_with_timeout(mut self, timeout: Option<u64>) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn with_retry(mut self, retry: ModelConfigRetry) -> Self {
        self.retry = Some(retry);
        self
    }

    pub fn maybe_with_retry(mut self, retry: Option<ModelConfigRetry>) -> Self {
        self.retry = retry;
        self
    }
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Request-time model selection and inference parameter overrides.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelConfigOverride {
    /// Override the provider (e.g. `"openai"`, `"anthropic"`).
    pub provider: Option<String>,
    /// Override the model name (e.g. `"gpt-4o"`, `"claude-sonnet-4-6"`).
    pub model: Option<String>,
    /// Override inference parameters. Fields set here take precedence over
    /// the agent identity defaults.
    pub inference_params: Option<InferenceParams>,
}

/// Retry policy for the model send handshake.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelConfigRetry {
    pub max_attempts: u32,
    pub initial_backoff: u64,
    pub max_backoff: u64,
}
