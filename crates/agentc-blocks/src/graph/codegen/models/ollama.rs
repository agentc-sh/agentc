// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use proc_macro2::TokenStream;
use quote::quote;

use crate::{
    config::fields::FieldsSpec, context::ResolvedContextProviderOllama,
    graph::codegen::models::ProviderKindCodeGen,
};

impl ProviderKindCodeGen for ResolvedContextProviderOllama {
    fn kind(&self) -> &'static str {
        "ollama"
    }

    fn imports(&self) -> TokenStream {
        quote! {
            use agentc_model::providers::ollama::{OllamaConfig, OllamaFactory};
        }
    }

    fn factory(&self) -> TokenStream {
        quote! { OllamaFactory }
    }

    fn config(&self, fields: &FieldsSpec, provider: &str) -> TokenStream {
        let base_url = self
            .base_url
            .as_ref()
            .and_then(|_| fields.config_accessor(&["provider", provider, "base_url"]))
            .map(|path| quote! { Some(#path.clone()) })
            .unwrap_or(quote! { None });

        quote! {
            OllamaConfig {
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
    fn imports_and_config_reference_the_ollama_factory() {
        let provider = ResolvedContextProviderOllama { base_url: None };

        assert!(
            provider
                .imports()
                .to_string()
                .contains("OllamaFactory")
        );
        assert_eq!(provider.factory().to_string(), "OllamaFactory");

        let rendered = provider
            .config(&FieldsSpec::new(vec![]), "ollama")
            .to_string()
            .replace(' ', "");
        assert!(rendered.contains("OllamaConfig{"));
    }
}
