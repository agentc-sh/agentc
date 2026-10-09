// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use proc_macro2::TokenStream;
use quote::quote;

use crate::{
    config::fields::FieldsSpec,
    context::ResolvedContextProviderHuggingFace,
    graph::codegen::models::ProviderKindCodeGen,
};

impl ProviderKindCodeGen for ResolvedContextProviderHuggingFace {
    fn kind(&self) -> &'static str {
        "huggingface"
    }

    fn imports(&self) -> TokenStream {
        quote! {
            use agentc_model::providers::huggingface::{HuggingFaceConfig, HuggingFaceFactory};
        }
    }

    fn factory(&self) -> TokenStream {
        quote! { HuggingFaceFactory }
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
            HuggingFaceConfig {
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
    fn imports_and_config_reference_the_huggingface_factory() {
        let provider = ResolvedContextProviderHuggingFace { api_key: None, base_url: None };

        assert!(
            provider
                .imports()
                .to_string()
                .contains("HuggingFaceFactory")
        );
        assert_eq!(provider.factory().to_string(), "HuggingFaceFactory");

        let rendered = provider
            .config(&FieldsSpec::new(vec![]), "huggingface")
            .to_string()
            .replace(' ', "");
        assert!(rendered.contains("HuggingFaceConfig{"));
    }
}
