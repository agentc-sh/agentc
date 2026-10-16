// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use agentc_compiler::generator::blocks::codegen::ToIdent;
use proc_macro2::TokenStream;
use quote::quote;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::BTreeMap;

use crate::{
    config::fields::{
        spec::{FieldsSpec, IntoFieldSpecs},
        types::{AsFieldValue, FieldValue, IntoTypeTokens},
    },
    context::{
        ResolvedContextProvider, ResolvedContextProviderAnthropic, ResolvedContextProviderGemini,
        ResolvedContextProviderHuggingFace, ResolvedContextProviderKind,
        ResolvedContextProviderOllama, ResolvedContextProviderOpenAi,
        ResolvedContextProviderOpenRouter, ResolvedContextProviderParams,
        ResolvedContextProviderXai,
    },
    types::ExpressionValue,
};

/// Registers every set inference parameter under `["provider", provider, slug, <field>]`.
///
/// The `slug` is either `"params"` for provider-level defaults or a model's slug for
/// per-model overrides. Kept as an extension trait so the provider impls below stay
/// focused on their own config shape.
trait ExtendParamFields {
    fn extend_param_fields(&self, fields: &mut FieldsSpec, provider: &str, slug: &str);
}

trait ExtendProviderFields {
    fn extend_provider_fields(&self, fields: &mut FieldsSpec, provider: &str);
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ResolvedContextProviderHeaders(BTreeMap<String, ExpressionValue<String>>);

impl ResolvedContextProviderHeaders {
    pub fn new(values: BTreeMap<String, ExpressionValue<String>>) -> Self {
        Self(values)
    }

    pub fn as_inner(&self) -> &BTreeMap<String, ExpressionValue<String>> {
        &self.0
    }

    pub fn as_inner_mut(&mut self) -> &mut BTreeMap<String, ExpressionValue<String>> {
        &mut self.0
    }

    pub fn into_inner(self) -> BTreeMap<String, ExpressionValue<String>> {
        self.0
    }

    pub fn uses_cel(&self) -> bool {
        self.0
            .values()
            .any(ExpressionValue::is_expression)
    }
}

impl AsFieldValue for ResolvedContextProviderHeaders {
    fn as_field_value(&self) -> FieldValue {
        FieldValue::constant(
            self.as_inner()
                .iter()
                .map(|(name, value)| {
                    (
                        name.clone(),
                        match value {
                            ExpressionValue::Literal(source) => {
                                json!({ "syntax": "literal", "source": source })
                            }
                            ExpressionValue::Expression(source) => {
                                json!({ "syntax": "cel", "source": source })
                            }
                        },
                    )
                })
                .collect::<serde_json::Map<_, _>>(),
        )
    }
}

impl IntoTypeTokens for ResolvedContextProviderHeaders {
    fn type_tokens() -> TokenStream {
        quote! { agentc_model::providers::rig::headers::Headers }
    }
}

impl ExtendParamFields for ResolvedContextProviderParams {
    fn extend_param_fields(&self, fields: &mut FieldsSpec, provider: &str, slug: &str) {
        if let Some(v) = &self.max_tokens {
            fields.push(&["provider", provider, slug, "max_tokens"], v);
        }
        if let Some(v) = &self.temperature {
            fields.push(&["provider", provider, slug, "temperature"], v);
        }
        if let Some(v) = &self.top_p {
            fields.push(&["provider", provider, slug, "top_p"], v);
        }
        if let Some(v) = &self.top_k {
            fields.push(&["provider", provider, slug, "top_k"], v);
        }
        if let Some(v) = &self.stop_sequences {
            fields.push(&["provider", provider, slug, "stop_sequences"], v);
        }
        if let Some(v) = &self.frequency_penalty {
            fields.push(&["provider", provider, slug, "frequency_penalty"], v);
        }
        if let Some(v) = &self.presence_penalty {
            fields.push(&["provider", provider, slug, "presence_penalty"], v);
        }
        if let Some(v) = &self.seed {
            fields.push(&["provider", provider, slug, "seed"], v);
        }
        if let Some(v) = &self.provider_params {
            fields.push(&["provider", provider, slug, "provider_params"], v);
        }
    }
}

impl IntoFieldSpecs for ResolvedContextProvider {
    fn extend_fields(&self, fields: &mut FieldsSpec) {
        let key = self.config_key();

        self.kind
            .extend_provider_fields(fields, key.as_str());

        if let Some(params) = &self.params {
            params.extend_param_fields(fields, key.as_str(), "params");
        }

        for model in self.models.iter().flatten() {
            if let Some(params) = &model.params {
                params.extend_param_fields(fields, key.as_str(), model.name.to_ident().as_str());
            }
        }
    }
}

impl ExtendProviderFields for ResolvedContextProviderKind {
    fn extend_provider_fields(&self, fields: &mut FieldsSpec, provider: &str) {
        match self {
            Self::Anthropic(kind) => kind.extend_provider_fields(fields, provider),
            Self::OpenAi(kind) => kind.extend_provider_fields(fields, provider),
            Self::Ollama(kind) => kind.extend_provider_fields(fields, provider),
            Self::OpenRouter(kind) => kind.extend_provider_fields(fields, provider),
            Self::Xai(kind) => kind.extend_provider_fields(fields, provider),
            Self::Gemini(kind) => kind.extend_provider_fields(fields, provider),
            Self::HuggingFace(kind) => kind.extend_provider_fields(fields, provider),
        }
    }
}

impl ExtendProviderFields for ResolvedContextProviderAnthropic {
    fn extend_provider_fields(&self, fields: &mut FieldsSpec, provider: &str) {
        if let Some(v) = &self.api_key {
            fields.push(&["provider", provider, "api_key"], v);
        }
        if let Some(v) = &self.base_url {
            fields.push(&["provider", provider, "base_url"], v);
        }
        if let Some(v) = &self.headers {
            fields.push(&["provider", provider, "headers"], v);
        }
    }
}

impl ExtendProviderFields for ResolvedContextProviderOpenAi {
    fn extend_provider_fields(&self, fields: &mut FieldsSpec, provider: &str) {
        if let Some(v) = &self.api_key {
            fields.push(&["provider", provider, "api_key"], v);
        }
        if let Some(v) = &self.base_url {
            fields.push(&["provider", provider, "base_url"], v);
        }
        if let Some(v) = &self.headers {
            fields.push(&["provider", provider, "headers"], v);
        }
    }
}

impl ExtendProviderFields for ResolvedContextProviderOllama {
    fn extend_provider_fields(&self, fields: &mut FieldsSpec, provider: &str) {
        if let Some(v) = &self.base_url {
            fields.push(&["provider", provider, "base_url"], v);
        }
        if let Some(v) = &self.headers {
            fields.push(&["provider", provider, "headers"], v);
        }
    }
}

impl ExtendProviderFields for ResolvedContextProviderOpenRouter {
    fn extend_provider_fields(&self, fields: &mut FieldsSpec, provider: &str) {
        if let Some(v) = &self.api_key {
            fields.push(&["provider", provider, "api_key"], v);
        }
        if let Some(v) = &self.headers {
            fields.push(&["provider", provider, "headers"], v);
        }
    }
}

impl ExtendProviderFields for ResolvedContextProviderXai {
    fn extend_provider_fields(&self, fields: &mut FieldsSpec, provider: &str) {
        if let Some(v) = &self.api_key {
            fields.push(&["provider", provider, "api_key"], v);
        }
        if let Some(v) = &self.headers {
            fields.push(&["provider", provider, "headers"], v);
        }
    }
}

impl ExtendProviderFields for ResolvedContextProviderGemini {
    fn extend_provider_fields(&self, fields: &mut FieldsSpec, provider: &str) {
        if let Some(v) = &self.api_key {
            fields.push(&["provider", provider, "api_key"], v);
        }
        if let Some(v) = &self.headers {
            fields.push(&["provider", provider, "headers"], v);
        }
    }
}

impl ExtendProviderFields for ResolvedContextProviderHuggingFace {
    fn extend_provider_fields(&self, fields: &mut FieldsSpec, provider: &str) {
        if let Some(v) = &self.api_key {
            fields.push(&["provider", provider, "api_key"], v);
        }
        if let Some(v) = &self.base_url {
            fields.push(&["provider", provider, "base_url"], v);
        }
        if let Some(v) = &self.headers {
            fields.push(&["provider", provider, "headers"], v);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{context::ResolvedContextProviderModel, types::RuntimeValue};

    fn empty_params() -> ResolvedContextProviderParams {
        ResolvedContextProviderParams {
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

    #[test]
    fn anthropic_registers_config_provider_params_and_model_params() {
        let provider = ResolvedContextProvider {
            name: "anthropic".to_string(),
            models: Some(vec![ResolvedContextProviderModel {
                name: "claude-3.5".to_string(),
                params: Some(ResolvedContextProviderParams {
                    temperature: Some(RuntimeValue::constant(0.7f64)),
                    ..empty_params()
                }),
            }]),
            params: Some(ResolvedContextProviderParams {
                max_tokens: Some(RuntimeValue::constant(1024u64)),
                ..empty_params()
            }),
            kind: ResolvedContextProviderKind::Anthropic(ResolvedContextProviderAnthropic {
                api_key: Some(RuntimeValue::secret_runtime("ANTHROPIC_KEY")),
                base_url: Some(RuntimeValue::constant("https://api".to_string())),
                headers: None,
            }),
        };

        let fields = FieldsSpec::collect_from(&provider);

        assert!(
            fields
                .get(&["provider", "anthropic", "api_key"])
                .is_some()
        );
        assert!(
            fields
                .get(&["provider", "anthropic", "base_url"])
                .is_some()
        );
        assert!(
            fields
                .get(&["provider", "anthropic", "params", "max_tokens"])
                .is_some()
        );
        // The model name is slugged into an identifier before it becomes a path segment.
        assert!(
            fields
                .get(&["provider", "anthropic", "claude_3_5", "temperature"])
                .is_some()
        );
    }

    #[test]
    fn ollama_registers_only_base_url_from_config() {
        let provider = ResolvedContextProvider {
            name: "ollama".to_string(),
            models: None,
            params: None,
            kind: ResolvedContextProviderKind::Ollama(ResolvedContextProviderOllama {
                base_url: Some(RuntimeValue::constant("http://localhost:11434".to_string())),
                headers: None,
            }),
        };

        let fields = FieldsSpec::collect_from(&provider);

        assert!(
            fields
                .get(&["provider", "ollama", "base_url"])
                .is_some()
        );
        assert!(
            fields
                .get(&["provider", "ollama", "api_key"])
                .is_none()
        );
    }

    #[test]
    fn xai_registers_api_key_but_has_no_base_url() {
        let provider = ResolvedContextProvider {
            name: "xai".to_string(),
            models: None,
            params: None,
            kind: ResolvedContextProviderKind::Xai(ResolvedContextProviderXai {
                api_key: Some(RuntimeValue::secret_runtime("XAI_KEY")),
                headers: None,
            }),
        };

        let fields = FieldsSpec::collect_from(&provider);

        assert!(
            fields
                .get(&["provider", "xai", "api_key"])
                .is_some()
        );
        assert!(
            fields
                .get(&["provider", "xai", "base_url"])
                .is_none()
        );
    }

    #[test]
    fn huggingface_registers_config_provider_params_and_model_params() {
        let provider = ResolvedContextProvider {
            name: "huggingface".to_string(),
            models: Some(vec![ResolvedContextProviderModel {
                name: "google/gemma-2-2b-it".to_string(),
                params: Some(ResolvedContextProviderParams {
                    max_tokens: Some(RuntimeValue::constant(1024u64)),
                    ..empty_params()
                }),
            }]),
            params: Some(ResolvedContextProviderParams {
                temperature: Some(RuntimeValue::constant(0.4f64)),
                ..empty_params()
            }),
            kind: ResolvedContextProviderKind::HuggingFace(ResolvedContextProviderHuggingFace {
                api_key: Some(RuntimeValue::secret_runtime("HUGGINGFACE_KEY")),
                base_url: Some(RuntimeValue::constant("https://router.example.com".to_string())),
                headers: None,
            }),
        };

        let fields = FieldsSpec::collect_from(&provider);

        assert!(
            fields
                .get(&["provider", "huggingface", "api_key"])
                .is_some()
        );
        assert!(
            fields
                .get(&["provider", "huggingface", "base_url"])
                .is_some()
        );
        assert!(
            fields
                .get(&["provider", "huggingface", "params", "temperature"])
                .is_some()
        );
        assert!(
            fields
                .get(&[
                    "provider",
                    "huggingface",
                    "google_gemma_2_2b_it",
                    "max_tokens",
                ])
                .is_some()
        );
    }

    #[test]
    fn provider_fields_are_keyed_by_the_provider_label() {
        let provider = ResolvedContextProvider {
            name: "cloud".to_string(),
            models: None,
            params: Some(ResolvedContextProviderParams {
                temperature: Some(RuntimeValue::constant(0.2f64)),
                ..empty_params()
            }),
            kind: ResolvedContextProviderKind::OpenAi(ResolvedContextProviderOpenAi {
                api_key: Some(RuntimeValue::secret_runtime("OPENAI_KEY")),
                base_url: None,
                headers: None,
            }),
        };

        let fields = FieldsSpec::collect_from(&provider);

        assert!(
            fields
                .get(&["provider", "cloud", "api_key"])
                .is_some()
        );
        assert!(
            fields
                .get(&["provider", "cloud", "params", "temperature"])
                .is_some()
        );
        assert!(
            fields
                .get(&["provider", "openai", "api_key"])
                .is_none()
        );
    }
}
