// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::types::RuntimeValue;

/// Resolved providers information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResolvedContextProvider {
    /// Configuration for Anthropic provider.
    Anthropic(ResolvedContextProviderAnthropic),
    /// Configuration for OpenAI provider.
    OpenAi(ResolvedContextProviderOpenAi),
    /// Configuration for Ollama provider.
    Ollama(ResolvedContextProviderOllama),
    /// Configuration for OpenRouter provider.
    OpenRouter(ResolvedContextProviderOpenRouter),
    /// Configuration for xAI provider.
    Xai(ResolvedContextProviderXai),
    /// Configuration for Gemini provider.
    Gemini(ResolvedContextProviderGemini),
    /// Configuration for Hugging Face provider.
    HuggingFace(ResolvedContextProviderHuggingFace),
}

/// Common inference parameters stored per provider or per model. Fields mirror the
/// manifest params block and retain their [`RuntimeValue`] wrappers so they can be
/// registered as config struct fields for runtime loading.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedContextProviderParams {
    pub max_tokens: Option<RuntimeValue<u64>>,
    pub temperature: Option<RuntimeValue<f64>>,
    pub top_p: Option<RuntimeValue<f64>>,
    pub top_k: Option<RuntimeValue<u32>>,
    pub stop_sequences: Option<RuntimeValue<Vec<String>>>,
    pub frequency_penalty: Option<RuntimeValue<f64>>,
    pub presence_penalty: Option<RuntimeValue<f64>>,
    pub seed: Option<RuntimeValue<u64>>,
    pub provider_params: Option<RuntimeValue<Value>>,
}

/// Resolved provider configuration for Anthropic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedContextProviderAnthropic {
    /// List of available models for this provider, if specified.
    pub models: Option<Vec<ResolvedContextProviderAnthropicModel>>,
    /// Provider-specific configuration options.
    pub config: Option<ResolvedContextProviderAnthropicConfig>,
    /// Provider-level inference parameter defaults.
    pub params: Option<ResolvedContextProviderParams>,
}

/// A single model entry in the Anthropic provider's models list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedContextProviderAnthropicModel {
    pub name: String,
    /// Model-specific inference parameter overrides. Falls back to provider params
    /// for any field not set here.
    pub params: Option<ResolvedContextProviderParams>,
}

/// Resolved configuration for Anthropic provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedContextProviderAnthropicConfig {
    /// Provider API key, if required.
    pub api_key: Option<RuntimeValue<String>>,
    /// Base URL for the provider's API, if applicable.
    pub base_url: Option<RuntimeValue<String>>,
}

/// Resolved provider configuration for OpenAI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedContextProviderOpenAi {
    /// List of available models for this provider, if specified.
    pub models: Option<Vec<ResolvedContextProviderOpenAiModel>>,
    /// Provider-specific configuration options.
    pub config: Option<ResolvedContextProviderOpenAiConfig>,
    /// Provider-level inference parameter defaults.
    pub params: Option<ResolvedContextProviderParams>,
}

/// A single model entry in the OpenAI provider's models list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedContextProviderOpenAiModel {
    pub name: String,
    /// Model-specific inference parameter overrides.
    pub params: Option<ResolvedContextProviderParams>,
}

/// Resolved configuration for OpenAI provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedContextProviderOpenAiConfig {
    /// Provider API key, if required.
    pub api_key: Option<RuntimeValue<String>>,
    /// Base URL for the provider's API, if applicable.
    pub base_url: Option<RuntimeValue<String>>,
}

/// Resolved provider configuration for Ollama.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedContextProviderOllama {
    /// List of available models for this provider, if specified.
    pub models: Option<Vec<ResolvedContextProviderOllamaModel>>,
    /// Provider-specific configuration options.
    pub config: Option<ResolvedContextProviderOllamaConfig>,
    /// Provider-level inference parameter defaults.
    pub params: Option<ResolvedContextProviderParams>,
}

/// A single model entry in the Ollama provider's models list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedContextProviderOllamaModel {
    pub name: String,
    /// Model-specific inference parameter overrides.
    pub params: Option<ResolvedContextProviderParams>,
}

/// Resolved configuration for Ollama provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedContextProviderOllamaConfig {
    /// Base URL for the provider's API, if applicable.
    pub base_url: Option<RuntimeValue<String>>,
}

/// Resolved provider configuration for OpenRouter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedContextProviderOpenRouter {
    /// List of available models for this provider, if specified.
    pub models: Option<Vec<ResolvedContextProviderOpenRouterModel>>,
    /// Provider-specific configuration options.
    pub config: Option<ResolvedContextProviderOpenRouterConfig>,
    /// Provider-level inference parameter defaults.
    pub params: Option<ResolvedContextProviderParams>,
}

/// A single model entry in the OpenRouter provider's models list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedContextProviderOpenRouterModel {
    pub name: String,
    /// Model-specific inference parameter overrides.
    pub params: Option<ResolvedContextProviderParams>,
}

/// Resolved configuration for OpenRouter provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedContextProviderOpenRouterConfig {
    /// Provider API key, if required.
    pub api_key: Option<RuntimeValue<String>>,
}

/// Resolved provider configuration for xAI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedContextProviderXai {
    /// List of available models for this provider, if specified.
    pub models: Option<Vec<ResolvedContextProviderXaiModel>>,
    /// Provider-specific configuration options.
    pub config: Option<ResolvedContextProviderXaiConfig>,
    /// Provider-level inference parameter defaults.
    pub params: Option<ResolvedContextProviderParams>,
}

/// A single model entry in the xAI provider's models list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedContextProviderXaiModel {
    pub name: String,
    /// Model-specific inference parameter overrides.
    pub params: Option<ResolvedContextProviderParams>,
}

/// Resolved configuration for xAI provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedContextProviderXaiConfig {
    /// Provider API key, if required.
    pub api_key: Option<RuntimeValue<String>>,
}

/// Resolved provider configuration for Gemini.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedContextProviderGemini {
    /// List of available models for this provider, if specified.
    pub models: Option<Vec<ResolvedContextProviderGeminiModel>>,
    /// Provider-specific configuration options.
    pub config: Option<ResolvedContextProviderGeminiConfig>,
    /// Provider-level inference parameter defaults.
    pub params: Option<ResolvedContextProviderParams>,
}

/// A single model entry in the Gemini provider's models list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedContextProviderGeminiModel {
    pub name: String,
    /// Model-specific inference parameter overrides.
    pub params: Option<ResolvedContextProviderParams>,
}

/// Resolved configuration for Gemini provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedContextProviderGeminiConfig {
    /// Provider API key, if required.
    pub api_key: Option<RuntimeValue<String>>,
}

/// Resolved provider configuration for Hugging Face.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedContextProviderHuggingFace {
    /// List of available models for this provider, if specified.
    pub models: Option<Vec<ResolvedContextProviderHuggingFaceModel>>,
    /// Provider-specific configuration options.
    pub config: Option<ResolvedContextProviderHuggingFaceConfig>,
    /// Provider-level inference parameter defaults.
    pub params: Option<ResolvedContextProviderParams>,
}

/// A single model entry in the Hugging Face provider's models list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedContextProviderHuggingFaceModel {
    pub name: String,
    /// Model-specific inference parameter overrides.
    pub params: Option<ResolvedContextProviderParams>,
}

/// Resolved configuration for Hugging Face provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedContextProviderHuggingFaceConfig {
    /// Provider API key, if required.
    pub api_key: Option<RuntimeValue<String>>,
    /// Base URL for the provider's API, if applicable.
    pub base_url: Option<RuntimeValue<String>>,
}
