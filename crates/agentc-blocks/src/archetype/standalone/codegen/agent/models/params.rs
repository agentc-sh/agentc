// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use proc_macro2::TokenStream;
use quote::quote;

use crate::archetype::standalone::fields::FieldsSpec;

/// Builds the field entries of an `agentc_model::types::inference::InferenceParams`
/// struct literal from the config accessors registered under
/// `["provider", provider, slug, <field>]`.
///
/// The `slug` is either `"params"` for provider-level defaults or a model's slug for
/// per-model overrides. Returns an empty vec when no inference parameter is configured
/// at that path, letting a provider omit the corresponding `.with_*_params(...)` call.
///
/// This is an opt-in helper: a provider whose inference parameters diverge from the
/// shared `InferenceParams` shape simply builds its registration without it.
pub struct InferenceParamsFields;

impl InferenceParamsFields {
    pub fn build(fields: &FieldsSpec, provider: &str, slug: &str) -> Vec<TokenStream> {
        [
            fields
                .config_accessor(&["provider", provider, slug, "max_tokens"])
                .map(|p| quote! { max_tokens: Some(#p), }),
            fields
                .config_accessor(&["provider", provider, slug, "temperature"])
                .map(|p| quote! { temperature: Some(#p), }),
            fields
                .config_accessor(&["provider", provider, slug, "top_p"])
                .map(|p| quote! { top_p: Some(#p), }),
            fields
                .config_accessor(&["provider", provider, slug, "top_k"])
                .map(|p| quote! { top_k: Some(#p), }),
            fields
                .config_accessor(&["provider", provider, slug, "stop_sequences"])
                .map(|p| quote! { stop_sequences: Some(#p.clone()), }),
            fields
                .config_accessor(&["provider", provider, slug, "frequency_penalty"])
                .map(|p| quote! { frequency_penalty: Some(#p), }),
            fields
                .config_accessor(&["provider", provider, slug, "presence_penalty"])
                .map(|p| quote! { presence_penalty: Some(#p), }),
            fields
                .config_accessor(&["provider", provider, slug, "seed"])
                .map(|p| quote! { seed: Some(#p), }),
            fields
                .config_accessor(&["provider", provider, slug, "provider_params"])
                .map(|p| quote! { provider_params: Some(#p.clone()), }),
        ]
        .into_iter()
        .flatten()
        .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::RuntimeValue;

    #[test]
    fn builds_an_entry_only_for_each_present_parameter() {
        let mut fields = FieldsSpec::new(vec![]);
        fields.push(
            &["provider", "anthropic", "params", "max_tokens"],
            &RuntimeValue::constant(256u64),
        );
        fields.push(
            &["provider", "anthropic", "params", "temperature"],
            &RuntimeValue::constant(0.5f64),
        );

        let built = InferenceParamsFields::build(&fields, "anthropic", "params");

        assert_eq!(built.len(), 2);

        let rendered = built
            .iter()
            .map(|t| t.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(rendered.contains("max_tokens"));
        assert!(rendered.contains("temperature"));
    }

    #[test]
    fn builds_nothing_when_no_parameters_are_registered() {
        let fields = FieldsSpec::new(vec![]);

        assert!(InferenceParamsFields::build(&fields, "anthropic", "params").is_empty());
    }
}
