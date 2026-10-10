// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use proc_macro2::TokenStream;
use quote::quote;

use crate::{
    config::fields::FieldsSpec, context::ResolvedContextProviderOpenRouter,
    graph::codegen::models::ProviderKindCodeGen,
};

impl ProviderKindCodeGen for ResolvedContextProviderOpenRouter {
    fn kind(&self) -> &'static str {
        "openrouter"
    }

    fn imports(&self) -> TokenStream {
        quote! {
            use agentc_model::providers::openrouter::{OpenRouterConfig, OpenRouterFactory};
        }
    }

    fn factory(&self) -> TokenStream {
        quote! { OpenRouterFactory }
    }

    fn config(&self, fields: &FieldsSpec, provider: &str) -> TokenStream {
        let api_key = self
            .api_key
            .as_ref()
            .and_then(|_| fields.config_accessor(&["provider", provider, "api_key"]))
            .map(|path| quote! { Some(#path.clone().into_inner()) })
            .unwrap_or(quote! { None });

        quote! {
            OpenRouterConfig {
                api_key: #api_key,
                ..Default::default()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imports_and_config_reference_the_openrouter_factory() {
        let provider = ResolvedContextProviderOpenRouter { api_key: None };

        assert!(
            provider
                .imports()
                .to_string()
                .contains("OpenRouterFactory")
        );
        assert_eq!(provider.factory().to_string(), "OpenRouterFactory");

        let rendered = provider
            .config(&FieldsSpec::new(vec![]), "openrouter")
            .to_string()
            .replace(' ', "");
        assert!(rendered.contains("OpenRouterConfig{"));
    }
}
