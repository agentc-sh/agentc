// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use proc_macro2::TokenStream;
use quote::quote;

use agentc_compiler::generator::blocks::codegen::ToIdent;

use crate::{
    context::ResolvedContextProviderHuggingFace,
    fields::FieldsSpec,
    graph::codegen::models::{ModelCodeGen, params::InferenceParamsFields},
};

impl ModelCodeGen for ResolvedContextProviderHuggingFace {
    fn imports(&self) -> TokenStream {
        quote! {
            use agentc_model::providers::huggingface::{HuggingFaceConfig, HuggingFaceFactory};
        }
    }

    fn registration(&self, fields: &FieldsSpec) -> TokenStream {
        let api_key = self
            .config
            .as_ref()
            .and_then(|c| c.api_key.as_ref())
            .and_then(|_| fields.config_accessor(&["provider", "huggingface", "api_key"]))
            .map(|path| quote! { Some(#path.clone().into_inner()) })
            .unwrap_or(quote! { None });

        let base_url = self
            .config
            .as_ref()
            .and_then(|c| c.base_url.as_ref())
            .and_then(|_| fields.config_accessor(&["provider", "huggingface", "base_url"]))
            .map(|path| quote! { Some(#path.clone()) })
            .unwrap_or(quote! { None });

        let constraints = self.models.as_ref().map(|models| {
            let names = models
                .iter()
                .map(|m| m.name.as_str())
                .collect::<Vec<_>>();
            quote! {
                .with_constraints(HuggingFaceFactory::provider(), [#(#names),*])
            }
        });

        let provider_params = InferenceParamsFields::build(fields, "huggingface", "params");
        let with_provider_params = if provider_params.is_empty() {
            quote! {}
        } else {
            quote! {
                .with_provider_params(
                    HuggingFaceFactory::provider(),
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
                    "huggingface",
                    model.name.to_ident().as_str(),
                );

                if model_params.is_empty() {
                    return None;
                }

                let name = &model.name;

                Some(quote! {
                    .with_model_params(
                        HuggingFaceFactory::provider(),
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
            .with_factory(HuggingFaceFactory)
            .with_config(HuggingFaceFactory::provider(), HuggingFaceConfig {
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

    #[test]
    fn imports_and_registration_reference_the_huggingface_factory() {
        let provider = ResolvedContextProviderHuggingFace {
            config: None,
            params: None,
            models: None,
        };

        assert!(
            provider
                .imports()
                .to_string()
                .contains("HuggingFaceFactory")
        );

        let rendered = provider
            .registration(&FieldsSpec::new(vec![]))
            .to_string()
            .replace(' ', "");
        assert!(rendered.contains("with_factory(HuggingFaceFactory)"));
        assert!(rendered.contains("HuggingFaceConfig{"));
    }
}
