// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use std::collections::HashMap;
use std::path::PathBuf;

use agentc_compiler::generator::{
    blocks::codegen::CodeGen, context::GenerationContext, errors::GeneratorError,
    extension::ExtensionRegistry,
};

use crate::{
    archetype::standalone::fields::FieldsSpec,
    context::{
        ResolvedContext, ResolvedContextAgentPromptMessageRole, ResolvedContextProvider,
        ResolvedContextSkillKind, ResolvedContextToolBashEnv, ResolvedContextToolBashFsKind,
        ResolvedContextToolJavascript, ResolvedContextToolKind, ResolvedContextToolMcpTransport,
        ResolvedContextToolPython, ResolvedContextToolPythonInterpreter,
    },
    types::RuntimeValue,
};

pub struct AgentRsCodeGen {
    pub fields: FieldsSpec,
}

impl AgentRsCodeGen {
    fn config_path(fields: &FieldsSpec, path: &[&str]) -> Option<TokenStream> {
        fields.get(path).map(|f| {
            f.path
                .iter()
                .fold(quote! { config }, |acc, seg| {
                    let ident = Ident::new(seg, Span::call_site());
                    quote! { #acc.#ident }
                })
        })
    }

    fn generate_model_registry(
        ctx: &ResolvedContext,
        fields: &FieldsSpec,
    ) -> Result<(Vec<TokenStream>, Vec<TokenStream>), GeneratorError> {
        let mut imports = Vec::new();
        let mut registrations = Vec::new();

        for provider in &ctx.providers {
            match provider {
                ResolvedContextProvider::Anthropic(anthropic) => {
                    imports.push(quote! {
                        use agentc_model::providers::anthropic::{AnthropicConfig, AnthropicFactory};
                    });

                    let api_key = anthropic
                        .config
                        .as_ref()
                        .and_then(|c| c.api_key.as_ref())
                        .and_then(|_| {
                            Self::config_path(fields, &["provider", "anthropic", "api_key"])
                        })
                        .map(|path| quote! { Some(#path.clone().into_inner()) })
                        .unwrap_or(quote! { None });

                    let base_url = anthropic
                        .config
                        .as_ref()
                        .and_then(|c| c.base_url.as_ref())
                        .and_then(|_| {
                            Self::config_path(fields, &["provider", "anthropic", "base_url"])
                        })
                        .map(|path| quote! { Some(#path.clone()) })
                        .unwrap_or(quote! { None });

                    let constraints = anthropic.models.as_ref().map(|models| {
                        let names = models
                            .iter()
                            .map(|m| m.name.as_str())
                            .collect::<Vec<_>>();
                        quote! {
                            .with_constraints(AnthropicFactory::provider(), [#(#names),*])
                        }
                    });

                    let provider_param_fields = [
                        Self::config_path(
                            fields,
                            &["provider", "anthropic", "params", "max_tokens"],
                        )
                        .map(|p| quote! { max_tokens: Some(#p), }),
                        Self::config_path(
                            fields,
                            &["provider", "anthropic", "params", "temperature"],
                        )
                        .map(|p| quote! { temperature: Some(#p), }),
                        Self::config_path(fields, &["provider", "anthropic", "params", "top_p"])
                            .map(|p| quote! { top_p: Some(#p), }),
                        Self::config_path(fields, &["provider", "anthropic", "params", "top_k"])
                            .map(|p| quote! { top_k: Some(#p), }),
                        Self::config_path(
                            fields,
                            &["provider", "anthropic", "params", "stop_sequences"],
                        )
                        .map(|p| quote! { stop_sequences: Some(#p.clone()), }),
                        Self::config_path(
                            fields,
                            &["provider", "anthropic", "params", "frequency_penalty"],
                        )
                        .map(|p| quote! { frequency_penalty: Some(#p), }),
                        Self::config_path(
                            fields,
                            &["provider", "anthropic", "params", "presence_penalty"],
                        )
                        .map(|p| quote! { presence_penalty: Some(#p), }),
                        Self::config_path(fields, &["provider", "anthropic", "params", "seed"])
                            .map(|p| quote! { seed: Some(#p), }),
                        Self::config_path(
                            fields,
                            &["provider", "anthropic", "params", "provider_params"],
                        )
                        .map(|p| quote! { provider_params: Some(#p.clone()), }),
                    ]
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>();

                    let with_provider_params = if provider_param_fields.is_empty() {
                        quote! {}
                    } else {
                        quote! {
                            .with_provider_params(
                                AnthropicFactory::provider(),
                                agentc_model::types::inference::InferenceParams {
                                    #(#provider_param_fields)*
                                    ..Default::default()
                                },
                            )
                        }
                    };

                    let with_model_params = anthropic
                        .models
                        .iter()
                        .flatten()
                        .filter_map(|model| {
                            let slug = model
                                .name
                                .chars()
                                .map(|c| if c.is_alphanumeric() { c } else { '_' })
                                .collect::<String>();

                            let model_param_fields = [
                                Self::config_path(
                                    fields,
                                    &["provider", "anthropic", &slug, "max_tokens"],
                                )
                                .map(|p| quote! { max_tokens: Some(#p), }),
                                Self::config_path(
                                    fields,
                                    &["provider", "anthropic", &slug, "temperature"],
                                )
                                .map(|p| quote! { temperature: Some(#p), }),
                                Self::config_path(
                                    fields,
                                    &["provider", "anthropic", &slug, "top_p"],
                                )
                                .map(|p| quote! { top_p: Some(#p), }),
                                Self::config_path(
                                    fields,
                                    &["provider", "anthropic", &slug, "top_k"],
                                )
                                .map(|p| quote! { top_k: Some(#p), }),
                                Self::config_path(
                                    fields,
                                    &["provider", "anthropic", &slug, "stop_sequences"],
                                )
                                .map(|p| quote! { stop_sequences: Some(#p.clone()), }),
                                Self::config_path(
                                    fields,
                                    &["provider", "anthropic", &slug, "frequency_penalty"],
                                )
                                .map(|p| quote! { frequency_penalty: Some(#p), }),
                                Self::config_path(
                                    fields,
                                    &["provider", "anthropic", &slug, "presence_penalty"],
                                )
                                .map(|p| quote! { presence_penalty: Some(#p), }),
                                Self::config_path(
                                    fields,
                                    &["provider", "anthropic", &slug, "seed"],
                                )
                                .map(|p| quote! { seed: Some(#p), }),
                                Self::config_path(
                                    fields,
                                    &["provider", "anthropic", &slug, "provider_params"],
                                )
                                .map(|p| quote! { provider_params: Some(#p.clone()), }),
                            ]
                            .into_iter()
                            .flatten()
                            .collect::<Vec<_>>();

                            if model_param_fields.is_empty() {
                                return None;
                            }

                            let name = &model.name;

                            Some(quote! {
                                .with_model_params(
                                    AnthropicFactory::provider(),
                                    #name,
                                    agentc_model::types::inference::InferenceParams {
                                        #(#model_param_fields)*
                                        ..Default::default()
                                    },
                                )
                            })
                        })
                        .collect::<Vec<_>>();

                    registrations.push(quote! {
                        .with_factory(AnthropicFactory)
                        .with_config(AnthropicFactory::provider(), AnthropicConfig {
                            api_key: #api_key,
                            base_url: #base_url,
                            ..Default::default()
                        })?
                        #constraints
                        #with_provider_params
                        #(#with_model_params)*
                    });
                }
                ResolvedContextProvider::OpenAi(openai) => {
                    imports.push(quote! {
                        use agentc_model::providers::openai::{OpenAiConfig, OpenAiFactory};
                    });

                    let api_key = openai
                        .config
                        .as_ref()
                        .and_then(|c| c.api_key.as_ref())
                        .and_then(|_| Self::config_path(fields, &["provider", "openai", "api_key"]))
                        .map(|path| quote! { Some(#path.clone().into_inner()) })
                        .unwrap_or(quote! { None });

                    let base_url = openai
                        .config
                        .as_ref()
                        .and_then(|c| c.base_url.as_ref())
                        .and_then(|_| {
                            Self::config_path(fields, &["provider", "openai", "base_url"])
                        })
                        .map(|path| quote! { Some(#path.clone()) })
                        .unwrap_or(quote! { None });

                    let constraints = openai.models.as_ref().map(|models| {
                        let names = models
                            .iter()
                            .map(|m| m.name.as_str())
                            .collect::<Vec<_>>();
                        quote! {
                            .with_constraints(OpenAiFactory::provider(), [#(#names),*])
                        }
                    });

                    let provider_param_fields = [
                        Self::config_path(fields, &["provider", "openai", "params", "max_tokens"])
                            .map(|p| quote! { max_tokens: Some(#p), }),
                        Self::config_path(fields, &["provider", "openai", "params", "temperature"])
                            .map(|p| quote! { temperature: Some(#p), }),
                        Self::config_path(fields, &["provider", "openai", "params", "top_p"])
                            .map(|p| quote! { top_p: Some(#p), }),
                        Self::config_path(fields, &["provider", "openai", "params", "top_k"])
                            .map(|p| quote! { top_k: Some(#p), }),
                        Self::config_path(
                            fields,
                            &["provider", "openai", "params", "stop_sequences"],
                        )
                        .map(|p| quote! { stop_sequences: Some(#p.clone()), }),
                        Self::config_path(
                            fields,
                            &["provider", "openai", "params", "frequency_penalty"],
                        )
                        .map(|p| quote! { frequency_penalty: Some(#p), }),
                        Self::config_path(
                            fields,
                            &["provider", "openai", "params", "presence_penalty"],
                        )
                        .map(|p| quote! { presence_penalty: Some(#p), }),
                        Self::config_path(fields, &["provider", "openai", "params", "seed"])
                            .map(|p| quote! { seed: Some(#p), }),
                        Self::config_path(
                            fields,
                            &["provider", "openai", "params", "provider_params"],
                        )
                        .map(|p| quote! { provider_params: Some(#p.clone()), }),
                    ]
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>();

                    let with_provider_params = if provider_param_fields.is_empty() {
                        quote! {}
                    } else {
                        quote! {
                            .with_provider_params(
                                OpenAiFactory::provider(),
                                agentc_model::types::inference::InferenceParams {
                                    #(#provider_param_fields)*
                                    ..Default::default()
                                },
                            )
                        }
                    };

                    let with_model_params = openai
                        .models
                        .iter()
                        .flatten()
                        .filter_map(|model| {
                            let slug = model
                                .name
                                .chars()
                                .map(|c| if c.is_alphanumeric() { c } else { '_' })
                                .collect::<String>();

                            let model_param_fields = [
                                Self::config_path(
                                    fields,
                                    &["provider", "openai", &slug, "max_tokens"],
                                )
                                .map(|p| quote! { max_tokens: Some(#p), }),
                                Self::config_path(
                                    fields,
                                    &["provider", "openai", &slug, "temperature"],
                                )
                                .map(|p| quote! { temperature: Some(#p), }),
                                Self::config_path(fields, &["provider", "openai", &slug, "top_p"])
                                    .map(|p| quote! { top_p: Some(#p), }),
                                Self::config_path(fields, &["provider", "openai", &slug, "top_k"])
                                    .map(|p| quote! { top_k: Some(#p), }),
                                Self::config_path(
                                    fields,
                                    &["provider", "openai", &slug, "stop_sequences"],
                                )
                                .map(|p| quote! { stop_sequences: Some(#p.clone()), }),
                                Self::config_path(
                                    fields,
                                    &["provider", "openai", &slug, "frequency_penalty"],
                                )
                                .map(|p| quote! { frequency_penalty: Some(#p), }),
                                Self::config_path(
                                    fields,
                                    &["provider", "openai", &slug, "presence_penalty"],
                                )
                                .map(|p| quote! { presence_penalty: Some(#p), }),
                                Self::config_path(fields, &["provider", "openai", &slug, "seed"])
                                    .map(|p| quote! { seed: Some(#p), }),
                                Self::config_path(
                                    fields,
                                    &["provider", "openai", &slug, "provider_params"],
                                )
                                .map(|p| quote! { provider_params: Some(#p.clone()), }),
                            ]
                            .into_iter()
                            .flatten()
                            .collect::<Vec<_>>();

                            if model_param_fields.is_empty() {
                                return None;
                            }

                            let name = &model.name;

                            Some(quote! {
                                .with_model_params(
                                    OpenAiFactory::provider(),
                                    #name,
                                    agentc_model::types::inference::InferenceParams {
                                        #(#model_param_fields)*
                                        ..Default::default()
                                    },
                                )
                            })
                        })
                        .collect::<Vec<_>>();

                    registrations.push(quote! {
                        .with_factory(OpenAiFactory)
                        .with_config(OpenAiFactory::provider(), OpenAiConfig {
                            api_key: #api_key,
                            base_url: #base_url,
                            ..Default::default()
                        })?
                        #constraints
                        #with_provider_params
                        #(#with_model_params)*
                    });
                }
                ResolvedContextProvider::Ollama(ollama) => {
                    imports.push(quote! {
                        use agentc_model::providers::ollama::{OllamaConfig, OllamaFactory};
                    });

                    let base_url = ollama
                        .config
                        .as_ref()
                        .and_then(|c| c.base_url.as_ref())
                        .and_then(|_| {
                            Self::config_path(fields, &["provider", "ollama", "base_url"])
                        })
                        .map(|path| quote! { Some(#path.clone()) })
                        .unwrap_or(quote! { None });

                    let constraints = ollama.models.as_ref().map(|models| {
                        let names = models
                            .iter()
                            .map(|m| m.name.as_str())
                            .collect::<Vec<_>>();
                        quote! {
                            .with_constraints(OllamaFactory::provider(), [#(#names),*])
                        }
                    });

                    let provider_param_fields = [
                        Self::config_path(fields, &["provider", "ollama", "params", "max_tokens"])
                            .map(|p| quote! { max_tokens: Some(#p), }),
                        Self::config_path(fields, &["provider", "ollama", "params", "temperature"])
                            .map(|p| quote! { temperature: Some(#p), }),
                        Self::config_path(fields, &["provider", "ollama", "params", "top_p"])
                            .map(|p| quote! { top_p: Some(#p), }),
                        Self::config_path(fields, &["provider", "ollama", "params", "top_k"])
                            .map(|p| quote! { top_k: Some(#p), }),
                        Self::config_path(
                            fields,
                            &["provider", "ollama", "params", "stop_sequences"],
                        )
                        .map(|p| quote! { stop_sequences: Some(#p.clone()), }),
                        Self::config_path(
                            fields,
                            &["provider", "ollama", "params", "frequency_penalty"],
                        )
                        .map(|p| quote! { frequency_penalty: Some(#p), }),
                        Self::config_path(
                            fields,
                            &["provider", "ollama", "params", "presence_penalty"],
                        )
                        .map(|p| quote! { presence_penalty: Some(#p), }),
                        Self::config_path(fields, &["provider", "ollama", "params", "seed"])
                            .map(|p| quote! { seed: Some(#p), }),
                        Self::config_path(
                            fields,
                            &["provider", "ollama", "params", "provider_params"],
                        )
                        .map(|p| quote! { provider_params: Some(#p.clone()), }),
                    ]
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>();

                    let with_provider_params = if provider_param_fields.is_empty() {
                        quote! {}
                    } else {
                        quote! {
                            .with_provider_params(
                                OllamaFactory::provider(),
                                agentc_model::types::inference::InferenceParams {
                                    #(#provider_param_fields)*
                                    ..Default::default()
                                },
                            )
                        }
                    };

                    let with_model_params = ollama
                        .models
                        .iter()
                        .flatten()
                        .filter_map(|model| {
                            let slug = model
                                .name
                                .chars()
                                .map(|c| if c.is_alphanumeric() { c } else { '_' })
                                .collect::<String>();

                            let model_param_fields = [
                                Self::config_path(
                                    fields,
                                    &["provider", "ollama", &slug, "max_tokens"],
                                )
                                .map(|p| quote! { max_tokens: Some(#p), }),
                                Self::config_path(
                                    fields,
                                    &["provider", "ollama", &slug, "temperature"],
                                )
                                .map(|p| quote! { temperature: Some(#p), }),
                                Self::config_path(fields, &["provider", "ollama", &slug, "top_p"])
                                    .map(|p| quote! { top_p: Some(#p), }),
                                Self::config_path(fields, &["provider", "ollama", &slug, "top_k"])
                                    .map(|p| quote! { top_k: Some(#p), }),
                                Self::config_path(
                                    fields,
                                    &["provider", "ollama", &slug, "stop_sequences"],
                                )
                                .map(|p| quote! { stop_sequences: Some(#p.clone()), }),
                                Self::config_path(
                                    fields,
                                    &["provider", "ollama", &slug, "frequency_penalty"],
                                )
                                .map(|p| quote! { frequency_penalty: Some(#p), }),
                                Self::config_path(
                                    fields,
                                    &["provider", "ollama", &slug, "presence_penalty"],
                                )
                                .map(|p| quote! { presence_penalty: Some(#p), }),
                                Self::config_path(fields, &["provider", "ollama", &slug, "seed"])
                                    .map(|p| quote! { seed: Some(#p), }),
                                Self::config_path(
                                    fields,
                                    &["provider", "ollama", &slug, "provider_params"],
                                )
                                .map(|p| quote! { provider_params: Some(#p.clone()), }),
                            ]
                            .into_iter()
                            .flatten()
                            .collect::<Vec<_>>();

                            if model_param_fields.is_empty() {
                                return None;
                            }

                            let name = &model.name;

                            Some(quote! {
                                .with_model_params(
                                    OllamaFactory::provider(),
                                    #name,
                                    agentc_model::types::inference::InferenceParams {
                                        #(#model_param_fields)*
                                        ..Default::default()
                                    },
                                )
                            })
                        })
                        .collect::<Vec<_>>();

                    registrations.push(quote! {
                        .with_factory(OllamaFactory)
                        .with_config(OllamaFactory::provider(), OllamaConfig {
                            base_url: #base_url,
                            ..Default::default()
                        })?
                        #constraints
                        #with_provider_params
                        #(#with_model_params)*
                    });
                }
                ResolvedContextProvider::OpenRouter(openrouter) => {
                    imports.push(quote! {
                        use agentc_model::providers::openrouter::{OpenRouterConfig, OpenRouterFactory};
                    });

                    let api_key = openrouter
                        .config
                        .as_ref()
                        .and_then(|c| c.api_key.as_ref())
                        .and_then(|_| {
                            Self::config_path(fields, &["provider", "openrouter", "api_key"])
                        })
                        .map(|path| quote! { Some(#path.clone().into_inner()) })
                        .unwrap_or(quote! { None });

                    let constraints = openrouter.models.as_ref().map(|models| {
                        let names = models
                            .iter()
                            .map(|m| m.name.as_str())
                            .collect::<Vec<_>>();
                        quote! {
                            .with_constraints(OpenRouterFactory::provider(), [#(#names),*])
                        }
                    });

                    let provider_param_fields = [
                        Self::config_path(
                            fields,
                            &["provider", "openrouter", "params", "max_tokens"],
                        )
                        .map(|p| quote! { max_tokens: Some(#p), }),
                        Self::config_path(
                            fields,
                            &["provider", "openrouter", "params", "temperature"],
                        )
                        .map(|p| quote! { temperature: Some(#p), }),
                        Self::config_path(fields, &["provider", "openrouter", "params", "top_p"])
                            .map(|p| quote! { top_p: Some(#p), }),
                        Self::config_path(fields, &["provider", "openrouter", "params", "top_k"])
                            .map(|p| quote! { top_k: Some(#p), }),
                        Self::config_path(
                            fields,
                            &["provider", "openrouter", "params", "stop_sequences"],
                        )
                        .map(|p| quote! { stop_sequences: Some(#p.clone()), }),
                        Self::config_path(
                            fields,
                            &["provider", "openrouter", "params", "frequency_penalty"],
                        )
                        .map(|p| quote! { frequency_penalty: Some(#p), }),
                        Self::config_path(
                            fields,
                            &["provider", "openrouter", "params", "presence_penalty"],
                        )
                        .map(|p| quote! { presence_penalty: Some(#p), }),
                        Self::config_path(fields, &["provider", "openrouter", "params", "seed"])
                            .map(|p| quote! { seed: Some(#p), }),
                        Self::config_path(
                            fields,
                            &["provider", "openrouter", "params", "provider_params"],
                        )
                        .map(|p| quote! { provider_params: Some(#p.clone()), }),
                    ]
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>();

                    let with_provider_params = if provider_param_fields.is_empty() {
                        quote! {}
                    } else {
                        quote! {
                            .with_provider_params(
                                OpenRouterFactory::provider(),
                                agentc_model::types::inference::InferenceParams {
                                    #(#provider_param_fields)*
                                    ..Default::default()
                                },
                            )
                        }
                    };

                    let with_model_params = openrouter
                        .models
                        .iter()
                        .flatten()
                        .filter_map(|model| {
                            let slug = model
                                .name
                                .chars()
                                .map(|c| if c.is_alphanumeric() { c } else { '_' })
                                .collect::<String>();

                            let model_param_fields = [
                                Self::config_path(
                                    fields,
                                    &["provider", "openrouter", &slug, "max_tokens"],
                                )
                                .map(|p| quote! { max_tokens: Some(#p), }),
                                Self::config_path(
                                    fields,
                                    &["provider", "openrouter", &slug, "temperature"],
                                )
                                .map(|p| quote! { temperature: Some(#p), }),
                                Self::config_path(
                                    fields,
                                    &["provider", "openrouter", &slug, "top_p"],
                                )
                                .map(|p| quote! { top_p: Some(#p), }),
                                Self::config_path(
                                    fields,
                                    &["provider", "openrouter", &slug, "top_k"],
                                )
                                .map(|p| quote! { top_k: Some(#p), }),
                                Self::config_path(
                                    fields,
                                    &["provider", "openrouter", &slug, "stop_sequences"],
                                )
                                .map(|p| quote! { stop_sequences: Some(#p.clone()), }),
                                Self::config_path(
                                    fields,
                                    &["provider", "openrouter", &slug, "frequency_penalty"],
                                )
                                .map(|p| quote! { frequency_penalty: Some(#p), }),
                                Self::config_path(
                                    fields,
                                    &["provider", "openrouter", &slug, "presence_penalty"],
                                )
                                .map(|p| quote! { presence_penalty: Some(#p), }),
                                Self::config_path(
                                    fields,
                                    &["provider", "openrouter", &slug, "seed"],
                                )
                                .map(|p| quote! { seed: Some(#p), }),
                                Self::config_path(
                                    fields,
                                    &["provider", "openrouter", &slug, "provider_params"],
                                )
                                .map(|p| quote! { provider_params: Some(#p.clone()), }),
                            ]
                            .into_iter()
                            .flatten()
                            .collect::<Vec<_>>();

                            if model_param_fields.is_empty() {
                                return None;
                            }

                            let name = &model.name;

                            Some(quote! {
                                .with_model_params(
                                    OpenRouterFactory::provider(),
                                    #name,
                                    agentc_model::types::inference::InferenceParams {
                                        #(#model_param_fields)*
                                        ..Default::default()
                                    },
                                )
                            })
                        })
                        .collect::<Vec<_>>();

                    registrations.push(quote! {
                        .with_factory(OpenRouterFactory)
                        .with_config(OpenRouterFactory::provider(), OpenRouterConfig {
                            api_key: #api_key,
                            ..Default::default()
                        })?
                        #constraints
                        #with_provider_params
                        #(#with_model_params)*
                    });
                }
            }
        }

        Ok((imports, registrations))
    }

    fn generate_tools(
        ctx: &ResolvedContext,
        fields: &FieldsSpec,
    ) -> Result<(Vec<TokenStream>, Vec<TokenStream>), GeneratorError> {
        let has_javascript_tools = ctx
            .tools
            .values()
            .any(|t| t.kind.is_javascript());
        let has_bash_tools = ctx
            .tools
            .values()
            .any(|t| t.kind.is_bash());
        let has_embedded_python_tools = ctx.tools.values().any(|t| {
            matches!(
                &t.kind,
                ResolvedContextToolKind::Python(py)
                    if matches!(py.interpreter, ResolvedContextToolPythonInterpreter::Embedded)
            )
        });

        let mut imports = Vec::new();
        let mut registrations = Vec::new();

        if has_javascript_tools {
            imports.push(quote! {
                use agentc_tools::javascript::{QuickJsRuntime, JavascriptTool};
            });
            registrations.extend(Self::generate_javascript_tools(ctx, fields)?);
        }

        if has_bash_tools {
            imports.push(quote! {
                use agentc_tools::bash::{BashTool, config::{CommandPolicy, EnvPolicy, FsPolicy, ExecLimits, NetworkPolicy}};
            });
            registrations.extend(Self::generate_bash_tools(ctx)?);
        }

        if has_embedded_python_tools {
            imports.push(quote! {
                use agentc_tools::python::{EmbeddedRuntime, PythonTool};
            });
            registrations.extend(Self::generate_embedded_python_tools(ctx, fields)?);
        }

        Ok((imports, registrations))
    }

    /// Emits one `QuickJsRuntime` binding per unique bundle path, then one
    /// `.with_tool(JavascriptTool::builder()...)` registration per JS tool.
    fn generate_javascript_tools(
        ctx: &ResolvedContext,
        fields: &FieldsSpec,
    ) -> Result<Vec<TokenStream>, GeneratorError> {
        let mut registrations = Vec::new();

        // Group tools by bundle path so each unique bundle shares one runtime.
        let mut by_bundle: HashMap<&str, Vec<(&str, &ResolvedContextToolJavascript)>> =
            HashMap::new();
        for (tool_name, tool) in &ctx.tools {
            if let ResolvedContextToolKind::Javascript(js) = &tool.kind {
                by_bundle
                    .entry(js.bundle_path.as_str())
                    .or_default()
                    .push((tool_name.as_str(), js));
            }
        }

        for (bundle_path, tools) in &by_bundle {
            // Derive a stable Rust identifier from the bundle path.
            let slug: String = bundle_path
                .chars()
                .map(|c| if c.is_alphanumeric() { c } else { '_' })
                .collect();

            let runtime_ident = Ident::new(&format!("js_runtime_{slug}"), Span::call_site());

            // Union of capability strings across all tools sharing this bundle.
            let caps: Vec<&str> = {
                let mut seen = std::collections::HashSet::new();
                ctx.tools
                    .iter()
                    .filter(|(_, t)| {
                        matches!(&t.kind, ResolvedContextToolKind::Javascript(js) if js.bundle_path == *bundle_path)
                    })
                    .flat_map(|(_, t)| t.capabilities.iter().map(String::as_str))
                    .filter(|c| seen.insert(*c))
                    .collect()
            };

            let caps_tokens = if caps.is_empty() {
                quote! {}
            } else {
                quote! { .capabilities([#(#caps),*]) }
            };

            registrations.push(quote! {
                #[allow(non_snake_case, nonstandard_style)]
                let #runtime_ident = std::sync::Arc::new(
                    QuickJsRuntime::builder()
                        .source(include_str!(#bundle_path).to_string())
                        #caps_tokens
                        .num_interpreters(4)
                        .shutdown(shutdown.clone())
                        .build()
                        .await?
                );
            });

            for (tool_name, js) in tools {
                let export_name = &js.export_name;

                let tool_caps: Vec<&str> = ctx
                    .tools
                    .get(*tool_name)
                    .map(|t| {
                        t.capabilities
                            .iter()
                            .map(String::as_str)
                            .collect()
                    })
                    .unwrap_or_default();

                let caps_call = if tool_caps.is_empty() {
                    quote! {}
                } else {
                    quote! { .capabilities([#(#tool_caps),*]) }
                };

                let build_tool = quote! {
                    JavascriptTool::builder()
                        .runtime(#runtime_ident.clone())
                        .export_name(#export_name)
                        #caps_call
                        .build()
                        .await?
                };

                let enabled_path = Self::config_path(fields, &["tool", tool_name, "enabled"]);

                if let Some(enabled) = enabled_path {
                    registrations.push(quote! {
                        if #enabled {
                            builder = builder.with_tool(#build_tool);
                        }
                    });
                } else {
                    registrations.push(quote! {
                        builder = builder.with_tool(#build_tool);
                    });
                }
            }
        }

        Ok(registrations)
    }

    /// Emits one `EmbeddedRuntime` binding per unique `site_packages_path`, then delegates
    /// per-tool registrations to [`Self::generate_python_tool_registrations`].
    ///
    /// Tools that share the same virtual environment (same `site_packages_path`) share one
    /// `EmbeddedRuntime` instance, mirroring how JS tools share a `JavascriptRuntime` per bundle.
    /// All distinct `project_path`s in a group are also frozen into the shared runtime.
    fn generate_embedded_python_tools(
        ctx: &ResolvedContext,
        fields: &FieldsSpec,
    ) -> Result<Vec<TokenStream>, GeneratorError> {
        let mut registrations = Vec::new();

        // Group embedded tools by site_packages_path so each unique venv shares one runtime.
        let mut by_site_packages: HashMap<&str, Vec<(&str, &ResolvedContextToolPython)>> =
            HashMap::new();
        for (tool_name, tool) in &ctx.tools {
            if let ResolvedContextToolKind::Python(py) = &tool.kind
                && matches!(py.interpreter, ResolvedContextToolPythonInterpreter::Embedded) {
                    by_site_packages
                        .entry(py.site_packages_path.as_str())
                        .or_default()
                        .push((tool_name.as_str(), py));
                }
        }

        for (site_packages_path, tools) in &by_site_packages {
            // Derive a stable Rust identifier from the site-packages path.
            let slug: String = site_packages_path
                .chars()
                .map(|c| if c.is_alphanumeric() { c } else { '_' })
                .collect();
            let runtime_ident =
                Ident::new(&format!("py_embedded_runtime_{slug}"), Span::call_site());

            // Collect the distinct project paths for all tools in this group so each
            // tool package's source is also frozen into the shared runtime.
            let mut project_paths: Vec<&str> = tools
                .iter()
                .map(|(_, py)| py.project_path.as_str())
                .collect();
            project_paths.sort_unstable();
            project_paths.dedup();

            registrations.push(Self::generate_embedded_python_runtime(
                site_packages_path,
                &project_paths,
                &runtime_ident,
            ));
            registrations.extend(Self::generate_python_tool_registrations(
                tools,
                &runtime_ident,
                ctx,
                fields,
            ));
        }

        Ok(registrations)
    }

    /// Emits the `EmbeddedRuntime::builder()...` binding for a single site-packages group.
    ///
    /// The `site_packages_path` (installed dependencies, including `agentc_tdk`) and each
    /// entry in `project_paths` (the tool package sources) are all frozen into the runtime
    /// so that all tool code and its dependencies are embedded in the binary at compile time.
    fn generate_embedded_python_runtime(
        site_packages_path: &str,
        project_paths: &[&str],
        runtime_ident: &Ident,
    ) -> TokenStream {
        let project_frozen = project_paths.iter().map(|path| {
            quote! {
                .frozen(agentc_tools::python::py_freeze!(dir = #path))
            }
        });

        quote! {
            #[allow(non_snake_case, nonstandard_style)]
            let #runtime_ident = std::sync::Arc::new(
                EmbeddedRuntime::builder()
                    .frozen(agentc_tools::python::py_freeze!(dir = #site_packages_path))
                    #(#project_frozen)*
                    .num_interpreters(4)
                    .channel_size(32)
                    .shutdown(shutdown.clone())
                    .build()?
            );
        }
    }

    /// Emits one `.with_tool(PythonTool::builder()...)` registration per Python tool.
    ///
    /// This helper is backend-agnostic: it takes the resolved `runtime_ident` produced by
    /// whichever runtime generator was called, so the same code runs for both the embedded
    /// and (future) static backends.
    fn generate_python_tool_registrations(
        tools: &[(&str, &ResolvedContextToolPython)],
        runtime_ident: &Ident,
        ctx: &ResolvedContext,
        fields: &FieldsSpec,
    ) -> Vec<TokenStream> {
        let mut registrations = Vec::new();

        for (tool_name, py) in tools {
            let module_name = py.module_name.as_str();

            let tool_caps: Vec<&str> = ctx
                .tools
                .get(*tool_name)
                .map(|t| {
                    t.capabilities
                        .iter()
                        .map(String::as_str)
                        .collect()
                })
                .unwrap_or_default();

            let caps_call = if tool_caps.is_empty() {
                quote! {}
            } else {
                quote! { .capabilities([#(#tool_caps),*]) }
            };

            let build_tool = quote! {
                PythonTool::builder()
                    .runtime(#runtime_ident.clone())
                    .module(#module_name)
                    .tool_name(#tool_name)
                    #caps_call
                    .build()
                    .await?
            };

            let enabled_path = Self::config_path(fields, &["tool", tool_name, "enabled"]);

            if let Some(enabled) = enabled_path {
                registrations.push(quote! {
                    if #enabled {
                        builder = builder.with_tool(#build_tool);
                    }
                });
            } else {
                registrations.push(quote! {
                    builder = builder.with_tool(#build_tool);
                });
            }
        }

        registrations
    }

    /// Emits one `.with_typed_tool(BashTool::builder()...)` registration per Bash tool.
    fn generate_bash_tools(ctx: &ResolvedContext) -> Result<Vec<TokenStream>, GeneratorError> {
        let mut registrations = Vec::new();

        for tool in ctx.tools.values() {
            let ResolvedContextToolKind::Bash(bash) = &tool.kind else {
                continue;
            };

            let commands = &bash.commands;

            let command_policy = if commands.is_empty() {
                quote! { CommandPolicy::Unrestricted }
            } else {
                quote! { CommandPolicy::Allow(vec![#(#commands.to_string()),*]) }
            };

            let fs_policy = match &bash.fs.kind {
                ResolvedContextToolBashFsKind::InMemory => quote! { FsPolicy::InMemory },
                ResolvedContextToolBashFsKind::Overlay(path) => quote! {
                    FsPolicy::Overlay(::std::path::PathBuf::from(#path))
                },
                ResolvedContextToolBashFsKind::ReadWrite(path) => quote! {
                    FsPolicy::ReadWrite(::std::path::PathBuf::from(#path))
                },
            };

            let env_policy = match &bash.env {
                ResolvedContextToolBashEnv::Empty => quote! { EnvPolicy::Empty },
                ResolvedContextToolBashEnv::Inherit => quote! { EnvPolicy::Inherit },
                ResolvedContextToolBashEnv::Allow(vars) => quote! {
                    EnvPolicy::Allow(vec![#(#vars.to_string()),*])
                },
                ResolvedContextToolBashEnv::Deny(vars) => quote! {
                    EnvPolicy::Deny(vec![#(#vars.to_string()),*])
                },
            };

            let max_execution_time_secs = bash.limits.max_execution_time_secs;
            let max_output_size = bash.limits.max_output_size;
            let max_command_count = bash.limits.max_command_count;
            let max_loop_iterations = bash.limits.max_loop_iterations;

            let network_enabled = bash.network.enabled;
            let allowed_url_prefixes = &bash.network.allowed_url_prefixes;
            let allowed_methods = &bash.network.allowed_methods;
            let max_redirects = bash.network.max_redirects;
            let max_response_size = bash.network.max_response_size;
            let network_timeout_secs = bash.network.network_timeout_secs;

            let cwd = &bash.fs.cwd;

            registrations.push(quote! {
                builder = builder.with_typed_tool(
                    BashTool::builder()
                        .command_policy(#command_policy)
                        .fs_policy(#fs_policy)
                        .env_policy(#env_policy)
                        .cwd(#cwd)
                        .limits(ExecLimits {
                            max_execution_time: ::std::time::Duration::from_secs(#max_execution_time_secs),
                            max_output_size: #max_output_size,
                            max_command_count: #max_command_count,
                            max_loop_iterations: #max_loop_iterations,
                        })
                        .network(NetworkPolicy {
                            enabled: #network_enabled,
                            allowed_url_prefixes: vec![#(#allowed_url_prefixes.to_string()),*],
                            allowed_methods: ::std::collections::HashSet::from([#(#allowed_methods.to_string()),*]),
                            max_redirects: #max_redirects,
                            max_response_size: #max_response_size,
                            timeout: ::std::time::Duration::from_secs(#network_timeout_secs),
                        })
                        .build()
                );
            });
        }

        Ok(registrations)
    }

    fn generate_skills(
        ctx: &ResolvedContext,
    ) -> Result<(Vec<TokenStream>, Vec<TokenStream>), GeneratorError> {
        if ctx.skills.is_empty() {
            return Ok((vec![], vec![]));
        }

        let imports = vec![quote! {
            use agentc_skills::{
                registry::SkillRegistryBuilder,
                builder::AgentBuilderSkillsExt,
                tools::run::MaterializationPolicy,
            };
        }];

        let mut with_static_calls = Vec::new();

        for skill in ctx.skills.values() {
            match &skill.kind {
                ResolvedContextSkillKind::Source(s) => {
                    let skill_md_path = &s.skill_md_path;
                    let resources: Vec<TokenStream> = s
                        .resources
                        .iter()
                        .map(|(rel, abs)| quote! { (#rel, include_str!(#abs)) })
                        .collect();

                    with_static_calls.push(quote! {
                        .with_static(include_str!(#skill_md_path), &[#(#resources),*])?
                    });
                }

                ResolvedContextSkillKind::Content(c) => {
                    let skill_md = format!(
                        "---\nname: {}\ndescription: {}\n---\n{}",
                        skill.name, c.description, c.content,
                    );
                    let resources: Vec<TokenStream> = c
                        .resources
                        .iter()
                        .map(|(rel, content)| quote! { (#rel, #content) })
                        .collect();

                    with_static_calls.push(quote! {
                        .with_static(#skill_md, &[#(#resources),*])?
                    });
                }
            }
        }

        let registrations = vec![quote! {
            builder = builder.with_skill_registry(
                SkillRegistryBuilder::default()
                    #(#with_static_calls)*
                    .build(),
                MaterializationPolicy::OnDemand,
            );
        }];

        Ok((imports, registrations))
    }

    fn generate_mcp_loader_calls(ctx: &ResolvedContext) -> TokenStream {
        let mut calls = Vec::<TokenStream>::new();

        for (name, tool) in &ctx.tools {
            let ResolvedContextToolKind::Mcp(mcp) = &tool.kind else {
                continue;
            };

            match &mcp.transport {
                ResolvedContextToolMcpTransport::Stdio { command, args, env } => {
                    calls.push(quote! {
                        .constant(
                            path!["mcp", "servers", #name, "type"],
                            serde_json::json!("stdio")
                        )
                    });

                    Self::push_mcp_rv_loader(
                        &["mcp", "servers", name, "command"],
                        command,
                        &mut calls,
                    );

                    for (i, arg) in args.iter().enumerate() {
                        Self::push_mcp_rv_loader_indexed(
                            &["mcp", "servers", name, "args"],
                            i,
                            arg,
                            &mut calls,
                        );
                    }

                    for (key, value) in env {
                        Self::push_mcp_rv_loader(
                            &["mcp", "servers", name, "env", key],
                            value,
                            &mut calls,
                        );
                    }
                }

                ResolvedContextToolMcpTransport::Http { url, auth_token, headers } => {
                    calls.push(quote! {
                        .constant(
                            path!["mcp", "servers", #name, "type"],
                            serde_json::json!("http")
                        )
                    });

                    Self::push_mcp_rv_loader(&["mcp", "servers", name, "url"], url, &mut calls);

                    if let Some(token) = auth_token {
                        Self::push_mcp_rv_loader(
                            &["mcp", "servers", name, "auth_token"],
                            token,
                            &mut calls,
                        );
                    }

                    for (key, value) in headers {
                        Self::push_mcp_rv_loader(
                            &["mcp", "servers", name, "headers", key],
                            value,
                            &mut calls,
                        );
                    }
                }
            }
        }

        quote! { #(#calls)* }
    }

    fn generate_mcp_mapper_fields(ctx: &ResolvedContext) -> TokenStream {
        let mut fields = Vec::<TokenStream>::new();

        for (name, tool) in &ctx.tools {
            let ResolvedContextToolKind::Mcp(mcp) = &tool.kind else {
                continue;
            };

            match &mcp.transport {
                ResolvedContextToolMcpTransport::Stdio { command, args, env } => {
                    Self::push_mcp_rv_mapper(
                        &["mcp", "servers", name, "command"],
                        command,
                        &mut fields,
                    );

                    for (i, arg) in args.iter().enumerate() {
                        Self::push_mcp_rv_mapper_indexed(
                            &["mcp", "servers", name, "args"],
                            i,
                            arg,
                            &mut fields,
                        );
                    }

                    for (key, value) in env {
                        Self::push_mcp_rv_mapper(
                            &["mcp", "servers", name, "env", key],
                            value,
                            &mut fields,
                        );
                    }
                }

                ResolvedContextToolMcpTransport::Http { url, auth_token, headers } => {
                    Self::push_mcp_rv_mapper(&["mcp", "servers", name, "url"], url, &mut fields);

                    if let Some(token) = auth_token {
                        Self::push_mcp_rv_mapper(
                            &["mcp", "servers", name, "auth_token"],
                            token,
                            &mut fields,
                        );
                    }

                    for (key, value) in headers {
                        Self::push_mcp_rv_mapper(
                            &["mcp", "servers", name, "headers", key],
                            value,
                            &mut fields,
                        );
                    }
                }
            }
        }

        quote! { #(#fields)* }
    }

    fn push_mcp_rv_loader(path: &[&str], rv: &RuntimeValue<String>, calls: &mut Vec<TokenStream>) {
        let path_segments = path.to_vec();

        match rv {
            RuntimeValue::Constant(value) => {
                calls.push(quote! {
                    .constant(
                        path![#(#path_segments),*],
                        serde_json::json!(#value)
                    )
                });
            }
            RuntimeValue::Runtime { default, .. } => {
                if let Some(default) = default {
                    calls.push(quote! {
                        .default(
                            path![#(#path_segments),*],
                            serde_json::json!(#default)
                        )
                    });
                }
            }
        }
    }

    fn push_mcp_rv_loader_indexed(
        base_path: &[&str],
        index: usize,
        rv: &RuntimeValue<String>,
        calls: &mut Vec<TokenStream>,
    ) {
        let base_segments = base_path.to_vec();

        match rv {
            RuntimeValue::Constant(value) => {
                calls.push(quote! {
                    .constant(
                        path![#(#base_segments),*, #index],
                        serde_json::json!(#value)
                    )
                });
            }
            RuntimeValue::Runtime { default, .. } => {
                if let Some(default) = default {
                    calls.push(quote! {
                        .default(
                            path![#(#base_segments),*, #index],
                            serde_json::json!(#default)
                        )
                    });
                }
            }
        }
    }

    fn push_mcp_rv_mapper(path: &[&str], rv: &RuntimeValue<String>, fields: &mut Vec<TokenStream>) {
        let path_segments = path.to_vec();

        if let RuntimeValue::Runtime { env, .. } = rv {
            fields.push(quote! {
                .field(path![#(#path_segments),*], #env)
            });
        }
    }

    fn push_mcp_rv_mapper_indexed(
        base_path: &[&str],
        index: usize,
        rv: &RuntimeValue<String>,
        fields: &mut Vec<TokenStream>,
    ) {
        let base_segments = base_path.to_vec();

        if let RuntimeValue::Runtime { env, .. } = rv {
            fields.push(quote! {
                .field(path![#(#base_segments),*, #index], #env)
            });
        }
    }

    fn generate_identity(
        ctx: &ResolvedContext,
        fields: &FieldsSpec,
    ) -> Result<TokenStream, GeneratorError> {
        let name = &ctx.agent_name;

        let provider = Self::config_path(fields, &["agent", "model", "provider"])
            .map(|p| quote! { #p.clone() })
            .unwrap_or_else(|| {
                let v = ctx
                    .agent
                    .model
                    .provider
                    .default_value()
                    .cloned()
                    .unwrap_or_default();
                quote! { #v.to_string() }
            });

        let model = Self::config_path(fields, &["agent", "model", "name"])
            .map(|p| quote! { #p.clone() })
            .unwrap_or_else(|| {
                let v = ctx
                    .agent
                    .model
                    .name
                    .default_value()
                    .cloned()
                    .unwrap_or_default();
                quote! { #v.to_string() }
            });

        let prompt = match &ctx.agent.prompt {
            None => quote! { PromptTemplate::default() },
            Some(messages) => {
                let parts = messages.iter().map(|message| {
                    let role = match message.role {
                        ResolvedContextAgentPromptMessageRole::System => quote! { Role::System },
                        ResolvedContextAgentPromptMessageRole::User => quote! { Role::User },
                        ResolvedContextAgentPromptMessageRole::Assistant => {
                            quote! { Role::Assistant }
                        }
                    };
                    let content = &message.content;

                    quote! { .with_part(#role, #content) }
                });

                quote! {
                    PromptTemplate::new()
                        #(#parts)*
                }
            }
        };

        let capabilities = Self::config_path(fields, &["agent", "capabilities"])
            .map(|path| quote! { CapabilitySet::from(#path.clone()) })
            .unwrap_or_else(|| quote! { CapabilitySet::empty() });

        let capability_policy = ctx
            .agent
            .capability_policy
            .as_ref()
            .and_then(|_| Self::config_path(fields, &["agent", "capability_policy"]))
            .map(|path| {
                quote! {
                    #path.parse::<CapabilityPolicy>().expect("invalid capability policy value")
                }
            })
            .unwrap_or_else(|| quote! { CapabilityPolicy::default() });

        Ok(quote! {
            AgentIdentity {
                name: #name.into(),
                provider: #provider.into(),
                model: #model.into(),
                prompt: #prompt,
                capabilities: #capabilities,
                capability_policy: #capability_policy,
            }
        })
    }
}

impl CodeGen<ResolvedContext> for AgentRsCodeGen {
    fn generate_files(
        &self,
        ctx: &GenerationContext<ResolvedContext>,
        registry: &ExtensionRegistry,
    ) -> Result<Vec<(PathBuf, TokenStream)>, GeneratorError> {
        let extra_use = registry
            .get("agent::use")
            .and_then(|s| s.parse::<TokenStream>().ok());

        let extra_tools = registry
            .get("agent::tools")
            .and_then(|s| s.parse::<TokenStream>().ok());

        let (model_imports, model_registrations) =
            Self::generate_model_registry(ctx, &self.fields)?;
        let (tool_imports, tool_registrations) = Self::generate_tools(ctx, &self.fields)?;
        let (skill_imports, skill_registrations) = Self::generate_skills(ctx)?;
        let agent_identity = Self::generate_identity(ctx, &self.fields)?;

        let source = quote! {
            use std::sync::Arc;
            use anyhow::Result;
            use tokio_util::sync::CancellationToken;

            use agentc_database::Database;
            use agentc_prompt::{
                compaction::TailWindow,
                counter::TiktokenCounter,
                template::{PromptTemplate, Role},
            };
            use agentc_model::registry::ModelRegistry;
            use agentc_agent::{
                agent::Agent,
                graph::checkpoint::GraphCheckpointer,
                types::{
                    identity::AgentIdentity,
                    capability::{CapabilitySet, CapabilityPolicy},
                },
            };
            use agentc_mcp::{
                builder::AgentBuilderMcpExt,
                config::{McpServerConfig, McpTransport},
                registry::McpRegistry,
            };
            use agentc_agent_react::{
                checkpoint::handle::SqlReActCheckpointStoreHandle,
                graph::ReActNode,
                types::{
                    event::Event,
                    message::Message,
                },
            };

            use crate::config::{Config, McpTransportConfig};

            #(#model_imports)*
            #(#tool_imports)*
            #(#skill_imports)*

            #extra_use

            pub async fn build_agent(
                db: Arc<Database>,
                config: &Config,
                shutdown: CancellationToken,
            ) -> Result<Agent<ReActNode, Event, Message>> {
                let model_registry = ModelRegistry::builder()
                    #(#model_registrations)*
                    .build();

                let mut builder = Agent::builder()
                    .with_graph(
                        ReActNode::graph()
                            .with_checkpointer(
                                GraphCheckpointer::new(
                                    SqlReActCheckpointStoreHandle::new(db)
                                )
                            )
                            .build()
                    )
                    .with_model_registry(model_registry)
                    .with_token_counter(TiktokenCounter::o200k_base())
                    .with_compaction_strategy(TailWindow);

                #(#tool_registrations)*
                #(#skill_registrations)*

                #extra_tools

                if !config.mcp.servers.is_empty() {
                    let mut mcp_builder = McpRegistry::builder();

                    for (name, transport) in &config.mcp.servers {
                        mcp_builder = mcp_builder.with_server(
                            McpServerConfig::new(name.clone(), match transport {
                                McpTransportConfig::Stdio { command, args, env } => McpTransport::Stdio {
                                    command: command.clone(),
                                    args: args.clone(),
                                    env: env.clone(),
                                },
                                McpTransportConfig::Http { url, auth_token, headers } => McpTransport::StreamableHttp {
                                    url: url.clone(),
                                    auth_token: auth_token.clone(),
                                    headers: headers.clone(),
                                },
                            })
                        );
                    }

                    builder = builder.with_mcp_registry(&mcp_builder.build().await?).await;
                }

                Ok(
                    builder
                        .with_identity(#agent_identity)
                        .build()?
                )
            }
        };

        Ok(vec![("src/agent.rs".into(), source)])
    }

    fn generate_contribution(
        &self,
        ctx: &GenerationContext<ResolvedContext>,
        point: &str,
    ) -> Result<TokenStream, GeneratorError> {
        match point {
            "config::loader" => Ok(Self::generate_mcp_loader_calls(ctx)),
            "config::mapper" => Ok(Self::generate_mcp_mapper_fields(ctx)),
            "tools::features" => {
                let has_javascript = ctx
                    .tools
                    .values()
                    .any(|t| t.kind.is_javascript());
                let has_bash = ctx
                    .tools
                    .values()
                    .any(|t| t.kind.is_bash());
                let has_embedded_python = ctx
                    .tools
                    .values()
                    .any(|t| matches!(
                        &t.kind,
                        ResolvedContextToolKind::Python(py)
                            if matches!(py.interpreter, ResolvedContextToolPythonInterpreter::Embedded)
                    ));

                let features = [
                    has_javascript.then(|| quote! { "javascript" }),
                    has_bash.then(|| quote! { "bash" }),
                    has_embedded_python.then(|| quote! { "python-embedded" }),
                ]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>();

                Ok(quote! { #(#features),* })
            }
            _ => Err(GeneratorError::unexpected(format!("Unknown extension point '{}'", point))),
        }
    }
}
