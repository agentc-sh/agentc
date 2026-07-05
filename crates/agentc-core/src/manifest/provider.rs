// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use sanitizer::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use validator::Validate;

use agentc_blocks::types::RuntimeValue;

#[derive(Debug, Clone, Serialize, Deserialize, Validate, Sanitizer)]
pub struct ManifestProvider {
    #[serde(default)]
    #[validate(nested)]
    pub anthropic: Option<ManifestProviderAnthropic>,
    #[serde(default)]
    #[validate(nested)]
    pub openai: Option<ManifestProviderOpenAi>,
    #[serde(default)]
    #[validate(nested)]
    pub ollama: Option<ManifestProviderOllama>,
    #[serde(default)]
    #[validate(nested)]
    pub openrouter: Option<ManifestProviderOpenRouter>,
    #[serde(default)]
    #[validate(nested)]
    pub xai: Option<ManifestProviderXai>,
    #[serde(default)]
    #[validate(nested)]
    pub gemini: Option<ManifestProviderGemini>,
}

/// Common inference parameters shared across all providers. All fields are optional
/// and support both compile-time constants and runtime environment variable loading
/// via [`RuntimeValue`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestProviderParams {
    #[serde(default)]
    pub max_tokens: Option<RuntimeValue<u64>>,
    #[serde(default)]
    pub temperature: Option<RuntimeValue<f64>>,
    #[serde(default)]
    pub top_p: Option<RuntimeValue<f64>>,
    #[serde(default)]
    pub top_k: Option<RuntimeValue<u32>>,
    #[serde(default)]
    pub stop_sequences: Option<RuntimeValue<Vec<String>>>,
    #[serde(default)]
    pub frequency_penalty: Option<RuntimeValue<f64>>,
    #[serde(default)]
    pub presence_penalty: Option<RuntimeValue<f64>>,
    #[serde(default)]
    pub seed: Option<RuntimeValue<u64>>,
    #[serde(default)]
    pub provider_params: Option<RuntimeValue<Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, Sanitizer)]
pub struct ManifestProviderAnthropic {
    #[serde(default)]
    pub models: Option<Vec<ManifestProviderAnthropicModel>>,
    #[serde(default)]
    pub config: Option<ManifestProviderAnthropicConfig>,
    #[serde(default)]
    pub params: Option<ManifestProviderParams>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
#[allow(clippy::large_enum_variant)]
pub enum ManifestProviderAnthropicModel {
    Name(String),
    Config(ManifestProviderAnthropicModelConfig),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestProviderAnthropicModelConfig {
    pub name: String,
    #[serde(default)]
    pub params: Option<ManifestProviderParams>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, Sanitizer)]
pub struct ManifestProviderAnthropicConfig {
    #[serde(default)]
    pub api_key: Option<RuntimeValue<String>>,
    #[serde(default)]
    pub base_url: Option<RuntimeValue<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, Sanitizer)]
pub struct ManifestProviderOpenAi {
    #[serde(default)]
    pub models: Option<Vec<ManifestProviderOpenAiModel>>,
    #[serde(default)]
    pub config: Option<ManifestProviderOpenAiConfig>,
    #[serde(default)]
    pub params: Option<ManifestProviderParams>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
#[allow(clippy::large_enum_variant)]
pub enum ManifestProviderOpenAiModel {
    Name(String),
    Config(ManifestProviderOpenAiModelConfig),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestProviderOpenAiModelConfig {
    pub name: String,
    #[serde(default)]
    pub params: Option<ManifestProviderParams>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, Sanitizer)]
pub struct ManifestProviderOpenAiConfig {
    #[serde(default)]
    pub api_key: Option<RuntimeValue<String>>,
    #[serde(default)]
    pub base_url: Option<RuntimeValue<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, Sanitizer)]
pub struct ManifestProviderOllama {
    #[serde(default)]
    pub models: Option<Vec<ManifestProviderOllamaModel>>,
    #[serde(default)]
    pub config: Option<ManifestProviderOllamaConfig>,
    #[serde(default)]
    pub params: Option<ManifestProviderParams>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
#[allow(clippy::large_enum_variant)]
pub enum ManifestProviderOllamaModel {
    Name(String),
    Config(ManifestProviderOllamaModelConfig),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestProviderOllamaModelConfig {
    pub name: String,
    #[serde(default)]
    pub params: Option<ManifestProviderParams>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, Sanitizer)]
pub struct ManifestProviderOllamaConfig {
    #[serde(default)]
    pub base_url: Option<RuntimeValue<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, Sanitizer)]
pub struct ManifestProviderOpenRouter {
    #[serde(default)]
    pub models: Option<Vec<ManifestProviderOpenRouterModel>>,
    #[serde(default)]
    pub config: Option<ManifestProviderOpenRouterConfig>,
    #[serde(default)]
    pub params: Option<ManifestProviderParams>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
#[allow(clippy::large_enum_variant)]
pub enum ManifestProviderOpenRouterModel {
    Name(String),
    Config(ManifestProviderOpenRouterModelConfig),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestProviderOpenRouterModelConfig {
    pub name: String,
    #[serde(default)]
    pub params: Option<ManifestProviderParams>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, Sanitizer)]
pub struct ManifestProviderOpenRouterConfig {
    #[serde(default)]
    pub api_key: Option<RuntimeValue<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, Sanitizer)]
pub struct ManifestProviderXai {
    #[serde(default)]
    pub models: Option<Vec<ManifestProviderXaiModel>>,
    #[serde(default)]
    pub config: Option<ManifestProviderXaiConfig>,
    #[serde(default)]
    pub params: Option<ManifestProviderParams>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
#[allow(clippy::large_enum_variant)]
pub enum ManifestProviderXaiModel {
    Name(String),
    Config(ManifestProviderXaiModelConfig),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestProviderXaiModelConfig {
    pub name: String,
    #[serde(default)]
    pub params: Option<ManifestProviderParams>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, Sanitizer)]
pub struct ManifestProviderXaiConfig {
    #[serde(default)]
    pub api_key: Option<RuntimeValue<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, Sanitizer)]
pub struct ManifestProviderGemini {
    #[serde(default)]
    pub models: Option<Vec<ManifestProviderGeminiModel>>,
    #[serde(default)]
    pub config: Option<ManifestProviderGeminiConfig>,
    #[serde(default)]
    pub params: Option<ManifestProviderParams>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
#[allow(clippy::large_enum_variant)]
pub enum ManifestProviderGeminiModel {
    Name(String),
    Config(ManifestProviderGeminiModelConfig),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestProviderGeminiModelConfig {
    pub name: String,
    #[serde(default)]
    pub params: Option<ManifestProviderParams>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, Sanitizer)]
pub struct ManifestProviderGeminiConfig {
    #[serde(default)]
    pub api_key: Option<RuntimeValue<String>>,
}
