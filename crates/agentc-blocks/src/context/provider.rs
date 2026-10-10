// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use convert_case::{Case, Casing};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::types::RuntimeValue;

/// Resolved providers information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResolvedContextProviderKind {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedContextProvider {
    pub name: String,
    pub models: Option<Vec<ResolvedContextProviderModel>>,
    pub params: Option<ResolvedContextProviderParams>,
    pub kind: ResolvedContextProviderKind,
}

impl ResolvedContextProvider {
    pub fn config_key(&self) -> String {
        if self
            .name
            .contains(|c: char| !c.is_alphanumeric() && c != '_')
        {
            self.name.to_case(Case::Snake)
        } else {
            self.name.clone()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedContextProviderModel {
    pub name: String,
    pub params: Option<ResolvedContextProviderParams>,
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
    /// Provider API key, if required.
    pub api_key: Option<RuntimeValue<String>>,
    /// Base URL for the provider's API, if applicable.
    pub base_url: Option<RuntimeValue<String>>,
}

/// Resolved provider configuration for OpenAI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedContextProviderOpenAi {
    /// Provider API key, if required.
    pub api_key: Option<RuntimeValue<String>>,
    /// Base URL for the provider's API, if applicable.
    pub base_url: Option<RuntimeValue<String>>,
}

/// Resolved provider configuration for Ollama.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedContextProviderOllama {
    /// Base URL for the provider's API, if applicable.
    pub base_url: Option<RuntimeValue<String>>,
}

/// Resolved provider configuration for OpenRouter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedContextProviderOpenRouter {
    /// Provider API key, if required.
    pub api_key: Option<RuntimeValue<String>>,
}

/// Resolved provider configuration for xAI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedContextProviderXai {
    /// Provider API key, if required.
    pub api_key: Option<RuntimeValue<String>>,
}

/// Resolved provider configuration for Gemini.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedContextProviderGemini {
    /// Provider API key, if required.
    pub api_key: Option<RuntimeValue<String>>,
}

/// Resolved provider configuration for Hugging Face.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedContextProviderHuggingFace {
    /// Provider API key, if required.
    pub api_key: Option<RuntimeValue<String>>,
    /// Base URL for the provider's API, if applicable.
    pub base_url: Option<RuntimeValue<String>>,
}
