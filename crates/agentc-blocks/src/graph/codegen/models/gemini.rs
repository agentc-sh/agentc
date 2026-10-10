// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use proc_macro2::TokenStream;
use quote::quote;

use crate::{
    config::fields::FieldsSpec, context::ResolvedContextProviderGemini,
    graph::codegen::models::ProviderKindCodeGen,
};

impl ProviderKindCodeGen for ResolvedContextProviderGemini {
    fn kind(&self) -> &'static str {
        "gemini"
    }

    fn imports(&self) -> TokenStream {
        quote! {
            use agentc_model::providers::gemini::{GeminiConfig, GeminiFactory};
        }
    }

    fn factory(&self) -> TokenStream {
        quote! { GeminiFactory }
    }

    fn config(&self, fields: &FieldsSpec, provider: &str) -> TokenStream {
        let api_key = self
            .api_key
            .as_ref()
            .and_then(|_| fields.config_accessor(&["provider", provider, "api_key"]))
            .map(|path| quote! { Some(#path.clone().into_inner()) })
            .unwrap_or(quote! { None });

        quote! {
            GeminiConfig {
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
    fn imports_and_config_reference_the_gemini_factory() {
        let provider = ResolvedContextProviderGemini { api_key: None };

        assert!(
            provider
                .imports()
                .to_string()
                .contains("GeminiFactory")
        );
        assert_eq!(provider.factory().to_string(), "GeminiFactory");

        let rendered = provider
            .config(&FieldsSpec::new(vec![]), "gemini")
            .to_string()
            .replace(' ', "");
        assert!(rendered.contains("GeminiConfig{"));
    }
}
