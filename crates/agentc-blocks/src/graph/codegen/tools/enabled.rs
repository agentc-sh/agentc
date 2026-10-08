// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use proc_macro2::TokenStream;
use quote::quote;

use crate::{config::fields::FieldsSpec, context::ResolvedContextTool};

pub struct ToolEnabledGuard<'a>(pub &'a ResolvedContextTool);

impl ToolEnabledGuard<'_> {
    pub fn wrap(&self, fields: &FieldsSpec, registration: TokenStream) -> TokenStream {
        match fields.config_accessor(&["tool", &self.0.config_key(), "enabled"]) {
            Some(enabled) => quote! {
                if #enabled {
                    #registration
                }
            },
            None => registration,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::{
        context::{ResolvedContextToolJavascript, ResolvedContextToolKind},
        types::RuntimeValue,
    };

    struct ToolEnabledGuardFixture;

    impl ToolEnabledGuardFixture {
        fn tool(name: &str) -> ResolvedContextTool {
            ResolvedContextTool {
                name: name.to_string(),
                description: None,
                enabled: RuntimeValue::constant(true),
                capabilities: vec![],
                config: HashMap::new(),
                kind: ResolvedContextToolKind::Javascript(ResolvedContextToolJavascript {
                    bundle_path: "/artifacts/pkg/dist/index.js".to_string(),
                    export_name: "Search".to_string(),
                }),
            }
        }
    }

    #[test]
    fn wraps_the_registration_in_the_enabled_field() {
        let tool = ToolEnabledGuardFixture::tool("search");

        assert_eq!(
            ToolEnabledGuard(&tool)
                .wrap(&FieldsSpec::collect_from(&tool), quote! { register(); })
                .to_string(),
            quote! {
                if config.tool.search.enabled {
                    register();
                }
            }
            .to_string(),
        );
    }

    #[test]
    fn reads_the_enabled_field_under_the_config_key() {
        let tool = ToolEnabledGuardFixture::tool("web-search");

        assert!(
            ToolEnabledGuard(&tool)
                .wrap(&FieldsSpec::collect_from(&tool), quote! { register(); })
                .to_string()
                .contains("if config . tool . web_search . enabled")
        );
    }

    #[test]
    fn returns_the_registration_unwrapped_without_an_enabled_field() {
        assert_eq!(
            ToolEnabledGuard(&ToolEnabledGuardFixture::tool("search"))
                .wrap(&FieldsSpec::new(vec![]), quote! { register(); })
                .to_string(),
            quote! { register(); }.to_string(),
        );
    }
}
