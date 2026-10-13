// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::{
    config::fields::types::{AsFieldValue, FieldValue},
    context::ResolvedContextNetworkUrlPattern,
};

impl AsFieldValue for ResolvedContextNetworkUrlPattern {
    fn as_field_value(&self) -> FieldValue {
        FieldValue::Map {
            entries: BTreeMap::from([
                ("protocol".to_string(), self.protocol.as_field_value()),
                ("hostname".to_string(), self.hostname.as_field_value()),
                ("port".to_string(), self.port.as_field_value()),
                ("pathname".to_string(), self.pathname.as_field_value()),
            ]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::RuntimeValue;
    use quote::quote;

    #[test]
    fn a_pattern_is_a_map_of_its_four_fields() {
        let tokens = ResolvedContextNetworkUrlPattern {
            hostname: RuntimeValue::constant(Some("a.test".to_string())),
            port: RuntimeValue::required_runtime("PORT"),
            ..Default::default()
        }
        .as_field_value()
        .tokens(&["p".into()]);

        assert_eq!(
            tokens
                .loader()
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            vec![
                quote! { .seal(path!["p"]) }.to_string(),
                quote! { .constant(path!["p", "hostname"], serde_json::json!("a.test")) }
                    .to_string(),
                quote! { .constant(path!["p", "pathname"], serde_json::json!(null)) }.to_string(),
                quote! { .constant(path!["p", "protocol"], serde_json::json!(null)) }.to_string(),
            ],
        );
        assert_eq!(
            tokens
                .mapper()
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            vec![quote! { .alias(path!["p", "port"], "PORT") }.to_string()],
        );
    }

    #[test]
    fn a_fixed_pattern_is_one_constant() {
        let tokens = ResolvedContextNetworkUrlPattern {
            hostname: RuntimeValue::constant(Some("a.test".to_string())),
            ..Default::default()
        }
        .as_field_value()
        .tokens(&["p".into()]);

        assert_eq!(
            tokens
                .loader()
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            vec![
                quote! {
                    .constant(
                        path!["p"],
                        serde_json::json!({
                            "hostname": "a.test",
                            "pathname": null,
                            "port": null,
                            "protocol": null
                        })
                    )
                }
                .to_string(),
            ],
        );
        assert!(tokens.mapper().is_empty());
    }
}
