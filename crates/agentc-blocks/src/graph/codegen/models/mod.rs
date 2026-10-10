// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

pub mod anthropic;
pub mod gemini;
pub mod huggingface;
pub mod ollama;
pub mod openai;
pub mod openrouter;
pub mod params;
pub mod xai;

use std::collections::HashSet;

use proc_macro2::TokenStream;
use quote::quote;

use agentc_compiler::generator::{blocks::codegen::ToIdent, errors::GeneratorError};

use crate::{
    config::fields::FieldsSpec,
    context::{ResolvedContext, ResolvedContextProvider, ResolvedContextProviderKind},
    graph::codegen::models::params::InferenceParamsFields,
};

/// Model-registry code generation for a single provider instance.
pub trait ModelCodeGen {
    /// The full `ModelRegistry` builder call chain that registers this provider.
    fn registration(&self, fields: &FieldsSpec) -> TokenStream;
}

pub trait ProviderKindCodeGen {
    fn kind(&self) -> &'static str;

    fn imports(&self) -> TokenStream;

    fn factory(&self) -> TokenStream;

    fn config(&self, fields: &FieldsSpec, provider: &str) -> TokenStream;
}

impl ProviderKindCodeGen for ResolvedContextProviderKind {
    fn kind(&self) -> &'static str {
        match self {
            Self::Anthropic(kind) => kind.kind(),
            Self::OpenAi(kind) => kind.kind(),
            Self::Ollama(kind) => kind.kind(),
            Self::OpenRouter(kind) => kind.kind(),
            Self::Xai(kind) => kind.kind(),
            Self::Gemini(kind) => kind.kind(),
            Self::HuggingFace(kind) => kind.kind(),
        }
    }

    fn imports(&self) -> TokenStream {
        match self {
            Self::Anthropic(kind) => kind.imports(),
            Self::OpenAi(kind) => kind.imports(),
            Self::Ollama(kind) => kind.imports(),
            Self::OpenRouter(kind) => kind.imports(),
            Self::Xai(kind) => kind.imports(),
            Self::Gemini(kind) => kind.imports(),
            Self::HuggingFace(kind) => kind.imports(),
        }
    }

    fn factory(&self) -> TokenStream {
        match self {
            Self::Anthropic(kind) => kind.factory(),
            Self::OpenAi(kind) => kind.factory(),
            Self::Ollama(kind) => kind.factory(),
            Self::OpenRouter(kind) => kind.factory(),
            Self::Xai(kind) => kind.factory(),
            Self::Gemini(kind) => kind.factory(),
            Self::HuggingFace(kind) => kind.factory(),
        }
    }

    fn config(&self, fields: &FieldsSpec, provider: &str) -> TokenStream {
        match self {
            Self::Anthropic(kind) => kind.config(fields, provider),
            Self::OpenAi(kind) => kind.config(fields, provider),
            Self::Ollama(kind) => kind.config(fields, provider),
            Self::OpenRouter(kind) => kind.config(fields, provider),
            Self::Xai(kind) => kind.config(fields, provider),
            Self::Gemini(kind) => kind.config(fields, provider),
            Self::HuggingFace(kind) => kind.config(fields, provider),
        }
    }
}

impl ModelCodeGen for ResolvedContextProvider {
    fn registration(&self, fields: &FieldsSpec) -> TokenStream {
        let name = &self.name;
        let key = self.config_key();
        let factory = self.kind.factory();
        let config = self.kind.config(fields, key.as_str());

        let constraints = self.models.as_ref().map(|models| {
            let names = models
                .iter()
                .map(|m| m.name.as_str())
                .collect::<Vec<_>>();
            quote! {
                .with_constraints(#name, [#(#names),*])
            }
        });

        let provider_params = InferenceParamsFields::build(fields, key.as_str(), "params");
        let with_provider_params = if provider_params.is_empty() {
            quote! {}
        } else {
            quote! {
                .with_provider_params(
                    #name,
                    agentc_model::types::inference::InferenceParams {
                        #(#provider_params)*
                        ..Default::default()
                    },
                )
            }
        };

        let with_model_params = self
            .models
            .iter()
            .flatten()
            .filter_map(|model| {
                let model_params = InferenceParamsFields::build(
                    fields,
                    key.as_str(),
                    model.name.to_ident().as_str(),
                );

                if model_params.is_empty() {
                    return None;
                }

                let model_name = &model.name;

                Some(quote! {
                    .with_model_params(
                        #name,
                        #model_name,
                        agentc_model::types::inference::InferenceParams {
                            #(#model_params)*
                            ..Default::default()
                        },
                    )
                })
            })
            .collect::<Vec<_>>();

        quote! {
            .with_provider::<#factory>(#name, #config)?
            #constraints
            #with_provider_params
            #(#with_model_params)*
        }
    }
}

/// Aggregates model-registry code across every provider present in the context.
pub struct ModelRegistryCodeGen;

impl ModelRegistryCodeGen {
    pub fn generate(
        ctx: &ResolvedContext,
        fields: &FieldsSpec,
    ) -> Result<(Vec<TokenStream>, Vec<TokenStream>), GeneratorError> {
        let mut kinds = HashSet::new();
        let mut imports = Vec::new();
        let mut registrations = Vec::new();

        for provider in ctx.providers.values() {
            if kinds.insert(provider.kind.kind()) {
                let factory = provider.kind.factory();

                imports.push(provider.kind.imports());
                registrations.push(quote! { .with_factory(#factory) });
            }

            registrations.push(provider.registration(fields));
        }

        Ok((imports, registrations))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        context::{
            ResolvedContextProviderAnthropic, ResolvedContextProviderModel,
            ResolvedContextProviderOpenAi, ResolvedContextProviderParams,
        },
        types::RuntimeValue,
    };

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
    fn registration_emits_provider_constraints_and_parameters() {
        // Fields must be registered for their accessors to resolve to `Some(...)`;
        // the model slug segment mirrors `"claude-3".to_ident()`.
        let mut fields = FieldsSpec::new(vec![]);

        fields.push(
            &["provider", "anthropic", "api_key"],
            &RuntimeValue::<String>::secret_runtime("KEY"),
        );
        fields.push(
            &["provider", "anthropic", "base_url"],
            &RuntimeValue::constant("https://api".to_string()),
        );
        fields.push(
            &["provider", "anthropic", "params", "max_tokens"],
            &RuntimeValue::constant(1024u64),
        );
        fields.push(
            &["provider", "anthropic", "params", "stop_sequences"],
            &RuntimeValue::constant(vec!["STOP".to_string()]),
        );
        fields.push(
            &["provider", "anthropic", "claude_3", "temperature"],
            &RuntimeValue::constant(0.5f64),
        );

        let provider = ResolvedContextProvider {
            name: "anthropic".to_string(),
            models: Some(vec![ResolvedContextProviderModel {
                name: "claude-3".to_string(),
                params: Some(ResolvedContextProviderParams {
                    temperature: Some(RuntimeValue::constant(0.5f64)),
                    ..empty_params()
                }),
            }]),
            params: Some(ResolvedContextProviderParams {
                max_tokens: Some(RuntimeValue::constant(1024u64)),
                stop_sequences: Some(RuntimeValue::constant(vec!["STOP".to_string()])),
                ..empty_params()
            }),
            kind: ResolvedContextProviderKind::Anthropic(ResolvedContextProviderAnthropic {
                api_key: Some(RuntimeValue::secret_runtime("KEY")),
                base_url: Some(RuntimeValue::constant("https://api".to_string())),
            }),
        };

        let rendered = provider
            .registration(&fields)
            .to_string()
            .replace(' ', "");

        assert!(rendered.contains(
            "with_provider::<AnthropicFactory>(\"anthropic\",AnthropicConfig{api_key:Some(config.provider.anthropic.api_key.clone().into_inner())"
        ));
        assert!(rendered.contains("with_constraints(\"anthropic\",[\"claude-3\"])"));
        assert!(rendered.contains(
            "with_provider_params(\"anthropic\",agentc_model::types::inference::InferenceParams{"
        ));
        assert!(rendered.contains("max_tokens:Some(config.provider.anthropic.params.max_tokens),"));
        // `stop_sequences` is cloned; scalar params are not.
        assert!(rendered.contains(
            "stop_sequences:Some(config.provider.anthropic.params.stop_sequences.clone()),"
        ));
        assert!(rendered.contains(
            "with_model_params(\"anthropic\",\"claude-3\",agentc_model::types::inference::InferenceParams{temperature:Some(config.provider.anthropic.claude_3.temperature),"
        ));
    }

    #[test]
    fn registration_is_keyed_by_the_provider_label() {
        let mut fields = FieldsSpec::new(vec![]);

        fields.push(
            &["provider", "cloud", "api_key"],
            &RuntimeValue::<String>::secret_runtime("OPENAI_KEY"),
        );

        let rendered = ResolvedContextProvider {
            name: "cloud".to_string(),
            models: Some(vec![ResolvedContextProviderModel {
                name: "gpt-4o".to_string(),
                params: None,
            }]),
            params: None,
            kind: ResolvedContextProviderKind::OpenAi(ResolvedContextProviderOpenAi {
                api_key: Some(RuntimeValue::secret_runtime("OPENAI_KEY")),
                base_url: None,
            }),
        }
        .registration(&fields)
        .to_string()
        .replace(' ', "");

        assert!(rendered.contains(
            "with_provider::<OpenAiFactory>(\"cloud\",OpenAiConfig{api_key:Some(config.provider.cloud.api_key.clone().into_inner()),base_url:None"
        ));
        assert!(rendered.contains("with_constraints(\"cloud\",[\"gpt-4o\"])"));
    }
}
