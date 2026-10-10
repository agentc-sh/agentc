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
    /// The chat history to use as context for the model.
    pub messages: ChatHistory,
    /// Tools available to the model for use in generating a response.
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

    pub fn merge_overrides(&mut self, params: InferenceParams) {
        self.max_tokens = params.max_tokens.or(self.max_tokens);
        self.temperature = params.temperature.or(self.temperature);
        self.top_p = params.top_p.or(self.top_p);
        self.top_k = params.top_k.or(self.top_k);
        self.stop_sequences = params
            .stop_sequences
            .or(self.stop_sequences.take());
        self.frequency_penalty = params
            .frequency_penalty
            .or(self.frequency_penalty);
        self.presence_penalty = params
            .presence_penalty
            .or(self.presence_penalty);
        self.seed = params.seed.or(self.seed);
        self.provider_params = params
            .provider_params
            .or(self.provider_params.take());
    }

    pub fn with_defaults(mut self, params: &InferenceParams) -> Self {
        self.merge_defaults(params);
        self
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn merge_overrides_sets_some_fields_and_keeps_none_fields() {
        let mut request = CompletionRequest::new(vec![]);

        request.temperature = Some(0.1);
        request.seed = Some(7);

        request.merge_overrides(InferenceParams {
            max_tokens: Some(256),
            temperature: Some(0.9),
            top_p: Some(0.5),
            top_k: Some(40),
            stop_sequences: Some(vec!["END".to_string()]),
            frequency_penalty: Some(0.2),
            presence_penalty: Some(0.3),
            seed: None,
            provider_params: Some(json!({ "user": "u" })),
        });

        assert_eq!(request.max_tokens, Some(256));
        assert_eq!(request.temperature, Some(0.9));
        assert_eq!(request.top_p, Some(0.5));
        assert_eq!(request.top_k, Some(40));
        assert_eq!(request.stop_sequences, Some(vec!["END".to_string()]));
        assert_eq!(request.frequency_penalty, Some(0.2));
        assert_eq!(request.presence_penalty, Some(0.3));
        assert_eq!(request.seed, Some(7));
        assert_eq!(request.provider_params, Some(json!({ "user": "u" })));
    }

    #[test]
    fn merge_overrides_win_over_merge_defaults() {
        let mut request = CompletionRequest::new(vec![]);

        request.merge_overrides(InferenceParams {
            temperature: Some(0.9),
            ..Default::default()
        });
        request.merge_defaults(&InferenceParams {
            max_tokens: Some(1024),
            temperature: Some(0.2),
            ..Default::default()
        });

        assert_eq!(request.temperature, Some(0.9));
        assert_eq!(request.max_tokens, Some(1024));
    }
}
