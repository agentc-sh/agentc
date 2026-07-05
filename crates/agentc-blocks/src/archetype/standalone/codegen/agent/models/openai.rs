// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use proc_macro2::TokenStream;
use quote::quote;

use agentc_compiler::generator::blocks::codegen::ToIdent;

use crate::{
    archetype::standalone::{
        codegen::agent::models::{params::InferenceParamsFields, ModelCodeGen},
        fields::FieldsSpec,
    },
    context::ResolvedContextProviderOpenAi,
};

impl ModelCodeGen for ResolvedContextProviderOpenAi {
    fn imports(&self) -> TokenStream {
        quote! {
            use agentc_model::providers::openai::{OpenAiConfig, OpenAiFactory};
        }
    }

    fn registration(&self, fields: &FieldsSpec) -> TokenStream {
        let api_key = self
            .config
            .as_ref()
            .and_then(|c| c.api_key.as_ref())
            .and_then(|_| fields.config_accessor(&["provider", "openai", "api_key"]))
            .map(|path| quote! { Some(#path.clone().into_inner()) })
            .unwrap_or(quote! { None });

        let base_url = self
            .config
            .as_ref()
            .and_then(|c| c.base_url.as_ref())
            .and_then(|_| fields.config_accessor(&["provider", "openai", "base_url"]))
            .map(|path| quote! { Some(#path.clone()) })
            .unwrap_or(quote! { None });

        let constraints = self.models.as_ref().map(|models| {
            let names = models
                .iter()
                .map(|m| m.name.as_str())
                .collect::<Vec<_>>();
            quote! {
                .with_constraints(OpenAiFactory::provider(), [#(#names),*])
            }
        });

        let provider_params = InferenceParamsFields::build(fields, "openai", "params");
        let with_provider_params = if provider_params.is_empty() {
            quote! {}
        } else {
            quote! {
                .with_provider_params(
                    OpenAiFactory::provider(),
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
                let model_params =
                    InferenceParamsFields::build(fields, "openai", model.name.to_ident().as_str());

                if model_params.is_empty() {
                    return None;
                }

                let name = &model.name;

                Some(quote! {
                    .with_model_params(
                        OpenAiFactory::provider(),
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
            .with_factory(OpenAiFactory)
            .with_config(OpenAiFactory::provider(), OpenAiConfig {
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
