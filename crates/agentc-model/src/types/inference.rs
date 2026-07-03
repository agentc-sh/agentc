// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Inference parameters for a model request. Used as per-provider and per-model defaults
/// in the [`ModelRegistry`](crate::registry::ModelRegistry) and as request-time overrides
/// via the model override struct in the agent input.
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InferenceParams {
    /// The maximum number of tokens to generate in the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u64>,
    /// Sampling temperature. Higher values produce more random output.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    /// Nucleus sampling probability mass. The model considers only tokens
    /// comprising the top `top_p` probability mass.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    /// Limits sampling to the top-k most likely tokens at each step.
    /// Not supported by all providers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
    /// Sequences at which the model will stop generating further tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
    /// Penalizes repeated tokens proportional to how many times they have
    /// already appeared. Not supported by all providers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f64>,
    /// Penalizes tokens that have appeared at all in the output so far.
    /// Not supported by all providers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f64>,
    /// Random seed for deterministic sampling. Not supported by all providers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,
    /// Provider-specific parameters serialized as a JSON value. Merged on top
    /// of any provider-level defaults set in the client config.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_params: Option<Value>,
}

impl InferenceParams {
    /// Merge another `InferenceParams` on top of this one. Fields set in
    /// `other` override fields in `self`.
    pub fn merge(self, other: InferenceParams) -> Self {
        Self {
            max_tokens: other.max_tokens.or(self.max_tokens),
            temperature: other.temperature.or(self.temperature),
            top_p: other.top_p.or(self.top_p),
            top_k: other.top_k.or(self.top_k),
            stop_sequences: other
                .stop_sequences
                .or(self.stop_sequences),
            frequency_penalty: other
                .frequency_penalty
                .or(self.frequency_penalty),
            presence_penalty: other
                .presence_penalty
                .or(self.presence_penalty),
            seed: other.seed.or(self.seed),
            provider_params: other
                .provider_params
                .or(self.provider_params),
        }
    }
}
