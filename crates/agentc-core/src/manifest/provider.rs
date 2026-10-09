// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

pub mod traits;

use sanitizer::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use validator::Validate;

use agentc_blocks::{
    context::{
        ResolvedContextProvider,
        ResolvedContextProviderAnthropic,
        ResolvedContextProviderGemini,
        ResolvedContextProviderHuggingFace,
        ResolvedContextProviderKind,
        ResolvedContextProviderModel,
        ResolvedContextProviderOllama,
        ResolvedContextProviderOpenAi,
        ResolvedContextProviderOpenRouter,
        ResolvedContextProviderParams,
        ResolvedContextProviderXai,
    },
    types::RuntimeValue,
};

use crate::manifest::{
    errors::ManifestError,
    interpolate::Interpolate,
    provider::traits::{ManifestProviderModel, ResolveProviderKind},
};

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ManifestProviderDefinition<M, C> {
    pub models: Option<Vec<M>>,
    pub config: Option<C>,
    pub params: Option<ManifestProviderParams>,
}

impl<M, C> Sanitizer for ManifestProviderDefinition<M, C> {
    fn sanitize(&mut self) {}
}

impl<M, C> ManifestProviderDefinition<M, C>
where
    M: ManifestProviderModel,
    Self: ResolveProviderKind,
{
    pub fn resolve(
        &self,
        name: &str,
        locals: &Value,
    ) -> Result<ResolvedContextProvider, ManifestError> {
        Ok(
            ResolvedContextProvider {
                name: name.to_string(),
                models: self
                    .models
                    .as_ref()
                    .map(|models| {
                        models
                            .iter()
                            .map(|model| {
                                Ok(
                                    ResolvedContextProviderModel {
                                        name: model
                                            .name()
                                            .to_string()
                                            .interpolate(locals)?,
                                        params: model
                                            .params()
                                            .map(|params| params.resolve(locals))
                                            .transpose()?,
                                    }
                                )
                            })
                            .collect::<Result<_, ManifestError>>()
                    })
                    .transpose()?,
                params: self
                    .params
                    .as_ref()
                    .map(|params| params.resolve(locals))
                    .transpose()?,
                kind: self.resolve_kind(locals)?,
            }
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Sanitizer)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ManifestProvider {
    Anthropic(
        ManifestProviderDefinition<ManifestProviderAnthropicModel, ManifestProviderAnthropicConfig>,
    ),
    #[serde(rename = "openai")]
    OpenAi(ManifestProviderDefinition<ManifestProviderOpenAiModel, ManifestProviderOpenAiConfig>),
    Ollama(ManifestProviderDefinition<ManifestProviderOllamaModel, ManifestProviderOllamaConfig>),
    #[serde(rename = "openrouter")]
    OpenRouter(
        ManifestProviderDefinition<
            ManifestProviderOpenRouterModel,
            ManifestProviderOpenRouterConfig,
        >,
    ),
    Xai(ManifestProviderDefinition<ManifestProviderXaiModel, ManifestProviderXaiConfig>),
    Gemini(ManifestProviderDefinition<ManifestProviderGeminiModel, ManifestProviderGeminiConfig>),
    #[serde(rename = "huggingface")]
    HuggingFace(
        ManifestProviderDefinition<
            ManifestProviderHuggingFaceModel,
            ManifestProviderHuggingFaceConfig,
        >,
    ),
}

impl ManifestProvider {
    pub fn resolve(
        &self,
        name: &str,
        locals: &Value,
    ) -> Result<ResolvedContextProvider, ManifestError> {
        match self {
            Self::Anthropic(definition) => definition.resolve(name, locals),
            Self::OpenAi(definition) => definition.resolve(name, locals),
            Self::Ollama(definition) => definition.resolve(name, locals),
            Self::OpenRouter(definition) => definition.resolve(name, locals),
            Self::Xai(definition) => definition.resolve(name, locals),
            Self::Gemini(definition) => definition.resolve(name, locals),
            Self::HuggingFace(definition) => definition.resolve(name, locals),
        }
    }
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

impl ManifestProviderParams {
    pub fn resolve(&self, locals: &Value) -> Result<ResolvedContextProviderParams, ManifestError> {
        Ok(
            ResolvedContextProviderParams {
                max_tokens: self.max_tokens.clone(),
                temperature: self.temperature.clone(),
                top_p: self.top_p.clone(),
                top_k: self.top_k.clone(),
                stop_sequences: self
                    .stop_sequences
                    .clone()
                    .interpolate(locals)?,
                frequency_penalty: self.frequency_penalty.clone(),
                presence_penalty: self.presence_penalty.clone(),
                seed: self.seed.clone(),
                provider_params: self
                    .provider_params
                    .clone()
                    .interpolate(locals)?,
            }
        )
    }
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

impl ManifestProviderModel for ManifestProviderAnthropicModel {
    fn name(&self) -> &str {
        match self {
            Self::Name(name) => name,
            Self::Config(config) => &config.name,
        }
    }

    fn params(&self) -> Option<&ManifestProviderParams> {
        match self {
            Self::Name(_) => None,
            Self::Config(config) => config.params.as_ref(),
        }
    }
}

impl ResolveProviderKind
    for ManifestProviderDefinition<ManifestProviderAnthropicModel, ManifestProviderAnthropicConfig>
{
    fn resolve_kind(&self, locals: &Value) -> Result<ResolvedContextProviderKind, ManifestError> {
        Ok(
            ResolvedContextProviderKind::Anthropic(ResolvedContextProviderAnthropic {
                api_key: self
                    .config
                    .as_ref()
                    .and_then(|config| config.api_key.clone())
                    .interpolate(locals)?,
                base_url: self
                    .config
                    .as_ref()
                    .and_then(|config| config.base_url.clone())
                    .interpolate(locals)?,
            })
        )
    }
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

impl ManifestProviderModel for ManifestProviderOpenAiModel {
    fn name(&self) -> &str {
        match self {
            Self::Name(name) => name,
            Self::Config(config) => &config.name,
        }
    }

    fn params(&self) -> Option<&ManifestProviderParams> {
        match self {
            Self::Name(_) => None,
            Self::Config(config) => config.params.as_ref(),
        }
    }
}

impl ResolveProviderKind
    for ManifestProviderDefinition<ManifestProviderOpenAiModel, ManifestProviderOpenAiConfig>
{
    fn resolve_kind(&self, locals: &Value) -> Result<ResolvedContextProviderKind, ManifestError> {
        Ok(
            ResolvedContextProviderKind::OpenAi(ResolvedContextProviderOpenAi {
                api_key: self
                    .config
                    .as_ref()
                    .and_then(|config| config.api_key.clone())
                    .interpolate(locals)?,
                base_url: self
                    .config
                    .as_ref()
                    .and_then(|config| config.base_url.clone())
                    .interpolate(locals)?,
            })
        )
    }
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

impl ManifestProviderModel for ManifestProviderOllamaModel {
    fn name(&self) -> &str {
        match self {
            Self::Name(name) => name,
            Self::Config(config) => &config.name,
        }
    }

    fn params(&self) -> Option<&ManifestProviderParams> {
        match self {
            Self::Name(_) => None,
            Self::Config(config) => config.params.as_ref(),
        }
    }
}

impl ResolveProviderKind
    for ManifestProviderDefinition<ManifestProviderOllamaModel, ManifestProviderOllamaConfig>
{
    fn resolve_kind(&self, locals: &Value) -> Result<ResolvedContextProviderKind, ManifestError> {
        Ok(
            ResolvedContextProviderKind::Ollama(ResolvedContextProviderOllama {
                base_url: self
                    .config
                    .as_ref()
                    .and_then(|config| config.base_url.clone())
                    .interpolate(locals)?,
            })
        )
    }
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

impl ManifestProviderModel for ManifestProviderOpenRouterModel {
    fn name(&self) -> &str {
        match self {
            Self::Name(name) => name,
            Self::Config(config) => &config.name,
        }
    }

    fn params(&self) -> Option<&ManifestProviderParams> {
        match self {
            Self::Name(_) => None,
            Self::Config(config) => config.params.as_ref(),
        }
    }
}

impl ResolveProviderKind
    for ManifestProviderDefinition<
        ManifestProviderOpenRouterModel,
        ManifestProviderOpenRouterConfig,
    >
{
    fn resolve_kind(&self, locals: &Value) -> Result<ResolvedContextProviderKind, ManifestError> {
        Ok(
            ResolvedContextProviderKind::OpenRouter(ResolvedContextProviderOpenRouter {
                api_key: self
                    .config
                    .as_ref()
                    .and_then(|config| config.api_key.clone())
                    .interpolate(locals)?,
            })
        )
    }
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

impl ManifestProviderModel for ManifestProviderXaiModel {
    fn name(&self) -> &str {
        match self {
            Self::Name(name) => name,
            Self::Config(config) => &config.name,
        }
    }

    fn params(&self) -> Option<&ManifestProviderParams> {
        match self {
            Self::Name(_) => None,
            Self::Config(config) => config.params.as_ref(),
        }
    }
}

impl ResolveProviderKind
    for ManifestProviderDefinition<ManifestProviderXaiModel, ManifestProviderXaiConfig>
{
    fn resolve_kind(&self, locals: &Value) -> Result<ResolvedContextProviderKind, ManifestError> {
        Ok(
            ResolvedContextProviderKind::Xai(ResolvedContextProviderXai {
                api_key: self
                    .config
                    .as_ref()
                    .and_then(|config| config.api_key.clone())
                    .interpolate(locals)?,
            })
        )
    }
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

impl ManifestProviderModel for ManifestProviderGeminiModel {
    fn name(&self) -> &str {
        match self {
            Self::Name(name) => name,
            Self::Config(config) => &config.name,
        }
    }

    fn params(&self) -> Option<&ManifestProviderParams> {
        match self {
            Self::Name(_) => None,
            Self::Config(config) => config.params.as_ref(),
        }
    }
}

impl ResolveProviderKind
    for ManifestProviderDefinition<ManifestProviderGeminiModel, ManifestProviderGeminiConfig>
{
    fn resolve_kind(&self, locals: &Value) -> Result<ResolvedContextProviderKind, ManifestError> {
        Ok(
            ResolvedContextProviderKind::Gemini(ResolvedContextProviderGemini {
                api_key: self
                    .config
                    .as_ref()
                    .and_then(|config| config.api_key.clone())
                    .interpolate(locals)?,
            })
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
#[allow(clippy::large_enum_variant)]
pub enum ManifestProviderHuggingFaceModel {
    Name(String),
    Config(ManifestProviderHuggingFaceModelConfig),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestProviderHuggingFaceModelConfig {
    pub name: String,
    #[serde(default)]
    pub params: Option<ManifestProviderParams>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, Sanitizer)]
pub struct ManifestProviderHuggingFaceConfig {
    #[serde(default)]
    pub api_key: Option<RuntimeValue<String>>,
    #[serde(default)]
    pub base_url: Option<RuntimeValue<String>>,
}

impl ManifestProviderModel for ManifestProviderHuggingFaceModel {
    fn name(&self) -> &str {
        match self {
            Self::Name(name) => name,
            Self::Config(config) => &config.name,
        }
    }

    fn params(&self) -> Option<&ManifestProviderParams> {
        match self {
            Self::Name(_) => None,
            Self::Config(config) => config.params.as_ref(),
        }
    }
}

impl ResolveProviderKind
    for ManifestProviderDefinition<
        ManifestProviderHuggingFaceModel,
        ManifestProviderHuggingFaceConfig,
    >
{
    fn resolve_kind(&self, locals: &Value) -> Result<ResolvedContextProviderKind, ManifestError> {
        Ok(
            ResolvedContextProviderKind::HuggingFace(ResolvedContextProviderHuggingFace {
                api_key: self
                    .config
                    .as_ref()
                    .and_then(|config| config.api_key.clone())
                    .interpolate(locals)?,
                base_url: self
                    .config
                    .as_ref()
                    .and_then(|config| config.base_url.clone())
                    .interpolate(locals)?,
            })
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::format::SpecFormat;

    #[test]
    fn parses_huggingface_provider_configuration() {
        let ManifestProvider::HuggingFace(huggingface) = SpecFormat::hcl()
            .deserialize_string::<ManifestProvider>(
                r#"
kind   = "huggingface"
models = ["google/gemma-2-2b-it"]

config {
  api_key  = "test-key"
  base_url = "https://router.example.com"
}

params {
  temperature = 0.4
}
"#,
            )
            .unwrap()
        else {
            panic!("provider should be HuggingFace");
        };
        assert!(matches!(
            huggingface.models.as_deref(),
            Some([ManifestProviderHuggingFaceModel::Name(name)])
                if name == "google/gemma-2-2b-it"
        ));
        assert!(huggingface.config.is_some());
        assert!(huggingface.params.is_some());
    }
}
