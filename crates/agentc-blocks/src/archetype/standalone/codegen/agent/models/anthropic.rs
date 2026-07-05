// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use proc_macro2::TokenStream;
use quote::quote;

use agentc_compiler::generator::blocks::codegen::ToIdent;

use crate::{
    archetype::standalone::{
        codegen::agent::models::{ModelCodeGen, params::InferenceParamsFields},
        fields::FieldsSpec,
    },
    context::ResolvedContextProviderAnthropic,
};

impl ModelCodeGen for ResolvedContextProviderAnthropic {
    fn imports(&self) -> TokenStream {
        quote! {
            use agentc_model::providers::anthropic::{AnthropicConfig, AnthropicFactory};
        }
    }

    fn registration(&self, fields: &FieldsSpec) -> TokenStream {
        let api_key = self
            .config
            .as_ref()
            .and_then(|c| c.api_key.as_ref())
            .and_then(|_| fields.config_accessor(&["provider", "anthropic", "api_key"]))
            .map(|path| quote! { Some(#path.clone().into_inner()) })
            .unwrap_or(quote! { None });

        let base_url = self
            .config
            .as_ref()
            .and_then(|c| c.base_url.as_ref())
            .and_then(|_| fields.config_accessor(&["provider", "anthropic", "base_url"]))
            .map(|path| quote! { Some(#path.clone()) })
            .unwrap_or(quote! { None });

        let constraints = self.models.as_ref().map(|models| {
            let names = models
                .iter()
                .map(|m| m.name.as_str())
                .collect::<Vec<_>>();
            quote! {
                .with_constraints(AnthropicFactory::provider(), [#(#names),*])
            }
        });

        let provider_params = InferenceParamsFields::build(fields, "anthropic", "params");
        let with_provider_params = if provider_params.is_empty() {
            quote! {}
        } else {
            quote! {
                .with_provider_params(
                    AnthropicFactory::provider(),
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
                    "anthropic",
                    model.name.to_ident().as_str(),
                );

                if model_params.is_empty() {
                    return None;
                }

                let name = &model.name;

                Some(quote! {
                    .with_model_params(
                        AnthropicFactory::provider(),
                        #name,
                        agentc_model::types::inference::InferenceParams {
                            #(#model_params)*
                            ..Default::default()
                        },
                    )
                })
            })
            .collect::<Vec<_>>();

        quote! {
            .with_factory(AnthropicFactory)
            .with_config(AnthropicFactory::provider(), AnthropicConfig {
                api_key: #api_key,
                base_url: #base_url,
                ..Default::default()
            })?
            #constraints
            #with_provider_params
            #(#with_model_params)*
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        context::{
            ResolvedContextProviderAnthropicConfig, ResolvedContextProviderAnthropicModel,
            ResolvedContextProviderParams,
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
    fn imports_reference_the_anthropic_factory_and_config() {
        let rendered =
            ResolvedContextProviderAnthropic { config: None, params: None, models: None }
                .imports()
                .to_string();

        assert!(rendered.contains("AnthropicConfig"));
        assert!(rendered.contains("AnthropicFactory"));
    }

    #[test]
    fn registration_emits_config_constraints_and_parameters() {
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

        let provider = ResolvedContextProviderAnthropic {
            config: Some(ResolvedContextProviderAnthropicConfig {
                api_key: Some(RuntimeValue::secret_runtime("KEY")),
                base_url: Some(RuntimeValue::constant("https://api".to_string())),
            }),
            params: Some(ResolvedContextProviderParams {
                max_tokens: Some(RuntimeValue::constant(1024u64)),
                stop_sequences: Some(RuntimeValue::constant(vec!["STOP".to_string()])),
                ..empty_params()
            }),
            models: Some(vec![ResolvedContextProviderAnthropicModel {
                name: "claude-3".to_string(),
                params: Some(ResolvedContextProviderParams {
                    temperature: Some(RuntimeValue::constant(0.5f64)),
                    ..empty_params()
                }),
            }]),
        };

        let rendered = provider
            .registration(&fields)
            .to_string()
            .replace(' ', "");

        assert!(rendered.contains("with_factory(AnthropicFactory)"));
        assert!(rendered.contains(
            "AnthropicConfig{api_key:Some(config.provider.anthropic.api_key.clone().into_inner())"
        ));
        assert!(rendered.contains("base_url:Some(config.provider.anthropic.base_url.clone())"));
        assert!(rendered.contains("with_constraints(AnthropicFactory::provider(),[\"claude-3\"])"));
        assert!(rendered.contains(
            "with_provider_params(AnthropicFactory::provider(),agentc_model::types::inference::InferenceParams{"
        ));
        assert!(rendered.contains("max_tokens:Some(config.provider.anthropic.params.max_tokens),"));
        // `stop_sequences` is cloned; scalar params are not.
        assert!(rendered.contains(
            "stop_sequences:Some(config.provider.anthropic.params.stop_sequences.clone()),"
        ));
        assert!(rendered.contains(
            "with_model_params(AnthropicFactory::provider(),\"claude-3\",agentc_model::types::inference::InferenceParams{temperature:Some(config.provider.anthropic.claude_3.temperature),"
        ));
    }
}
