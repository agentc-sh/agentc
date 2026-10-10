// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use proc_macro2::TokenStream;
use quote::quote;

use crate::{
    config::fields::FieldsSpec, context::ResolvedContextProviderAnthropic,
    graph::codegen::models::ProviderKindCodeGen,
};

impl ProviderKindCodeGen for ResolvedContextProviderAnthropic {
    fn kind(&self) -> &'static str {
        "anthropic"
    }

    fn imports(&self) -> TokenStream {
        quote! {
            use agentc_model::providers::anthropic::{AnthropicConfig, AnthropicFactory};
        }
    }

    fn factory(&self) -> TokenStream {
        quote! { AnthropicFactory }
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
            AnthropicConfig {
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
    use crate::types::RuntimeValue;

    #[test]
    fn imports_reference_the_anthropic_factory_and_config() {
        let rendered = ResolvedContextProviderAnthropic { api_key: None, base_url: None }
            .imports()
            .to_string();

        assert!(rendered.contains("AnthropicConfig"));
        assert!(rendered.contains("AnthropicFactory"));
    }

    #[test]
    fn config_emits_api_key_and_base_url() {
        let mut fields = FieldsSpec::new(vec![]);

        fields.push(
            &["provider", "anthropic", "api_key"],
            &RuntimeValue::<String>::secret_runtime("KEY"),
        );
        fields.push(
            &["provider", "anthropic", "base_url"],
            &RuntimeValue::constant("https://api".to_string()),
        );

        let rendered = ResolvedContextProviderAnthropic {
            api_key: Some(RuntimeValue::secret_runtime("KEY")),
            base_url: Some(RuntimeValue::constant("https://api".to_string())),
        }
        .config(&fields, "anthropic")
        .to_string()
        .replace(' ', "");

        assert!(rendered.contains(
            "AnthropicConfig{api_key:Some(config.provider.anthropic.api_key.clone().into_inner())"
        ));
        assert!(rendered.contains("base_url:Some(config.provider.anthropic.base_url.clone())"));
    }
}
