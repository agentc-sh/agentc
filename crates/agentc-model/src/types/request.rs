// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use serde_json::Value;

use crate::types::{
    inference::InferenceParams,
    message::{ChatHistory, ChatMessage},
    tools::ToolSpec,
};

/// A completion request. Constructed via the fluent builder chain on
/// [`CompletionModel`](crate::traits::CompletionModel) and consumed by
/// [`CompletionModel::send`](crate::traits::CompletionModel::send).
#[derive(Debug, Clone)]
pub struct CompletionRequest {
    pub messages: ChatHistory,
    pub tools: Vec<ToolSpec>,
    /// Maximum number of tokens to generate.
    pub max_tokens: Option<u64>,
    /// Sampling temperature. Higher values produce more random output.
    pub temperature: Option<f64>,
    /// Nucleus sampling threshold. The model considers only the tokens comprising
    /// the top `top_p` probability mass.
    pub top_p: Option<f64>,
    /// Limits the model to the `top_k` most likely next tokens at each step.
    pub top_k: Option<u32>,
    /// Sequences at which the model will stop generating further tokens.
    pub stop_sequences: Option<Vec<String>>,
    /// Penalizes tokens proportional to how often they have already appeared,
    /// reducing repetition of specific tokens.
    pub frequency_penalty: Option<f64>,
    /// Penalizes tokens that have appeared at all, encouraging the model to
    /// introduce new topics.
    pub presence_penalty: Option<f64>,
    /// Seed for deterministic sampling. Not supported by all providers.
    pub seed: Option<u64>,
    /// Provider-specific parameters serialized as a JSON value. These are
    /// deserialized by the provider implementation and merged on top of any
    /// provider-level defaults set in the client config.
    pub provider_params: Option<Value>,
}

impl CompletionRequest {
    pub fn new(messages: Vec<ChatMessage>) -> Self {
        Self {
            messages: ChatHistory::new(messages),
            tools: vec![],
            max_tokens: None,
            temperature: None,
            top_p: None,
            top_k: None,
            stop_sequences: None,
            frequency_penalty: None,
            presence_penalty: None,
            seed: None,
            provider_params: None,
        }
    }

    /// Fill any `None` fields in this request with values from the given [`InferenceParams`].
    /// Fields already set on the request are left unchanged.
    pub fn merge_defaults(&mut self, params: &InferenceParams) {
        self.max_tokens = self.max_tokens.or(params.max_tokens);
        self.temperature = self.temperature.or(params.temperature);
        self.top_p = self.top_p.or(params.top_p);
        self.top_k = self.top_k.or(params.top_k);
        self.stop_sequences = self
            .stop_sequences
            .clone()
            .or_else(|| params.stop_sequences.clone());
        self.frequency_penalty = self
            .frequency_penalty
            .or(params.frequency_penalty);
        self.presence_penalty = self
            .presence_penalty
            .or(params.presence_penalty);
        self.seed = self.seed.or(params.seed);
        self.provider_params = self
            .provider_params
            .clone()
            .or_else(|| params.provider_params.clone());
    }

    pub fn with_defaults(mut self, params: &InferenceParams) -> Self {
        self.merge_defaults(params);
        self
    }
}
