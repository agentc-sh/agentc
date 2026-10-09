// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use proc_macro2::TokenStream;
use quote::quote;

use crate::{
    config::fields::FieldsSpec,
    context::ResolvedContextProviderOpenAi,
    graph::codegen::models::ProviderKindCodeGen,
};

impl ProviderKindCodeGen for ResolvedContextProviderOpenAi {
    fn kind(&self) -> &'static str {
        "openai"
    }

    fn imports(&self) -> TokenStream {
        quote! {
            use agentc_model::providers::openai::{OpenAiConfig, OpenAiFactory};
        }
    }

    fn factory(&self) -> TokenStream {
        quote! { OpenAiFactory }
    }

    fn config(&self, fields: &FieldsSpec, provider: &str) -> TokenStream {
        let api_key = self
            .api_key
            .as_ref()
            .and_then(|_| fields.config_accessor(&["provider", provider, "api_key"]))
            .map(|path| quote! { Some(#path.clone().into_inner()) })
            .unwrap_or(quote! { None });

        let base_url = self
            .base_url
            .as_ref()
            .and_then(|_| fields.config_accessor(&["provider", provider, "base_url"]))
            .map(|path| quote! { Some(#path.clone()) })
            .unwrap_or(quote! { None });

        quote! {
            OpenAiConfig {
                api_key: #api_key,
                base_url: #base_url,
                ..Default::default()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imports_and_config_reference_the_openai_factory() {
        let provider = ResolvedContextProviderOpenAi { api_key: None, base_url: None };

        assert!(
            provider
                .imports()
                .to_string()
                .contains("OpenAiFactory")
        );
        assert_eq!(provider.factory().to_string(), "OpenAiFactory");

        let rendered = provider
            .config(&FieldsSpec::new(vec![]), "openai")
            .to_string()
            .replace(' ', "");
        assert!(rendered.contains("OpenAiConfig{"));
    }
}
