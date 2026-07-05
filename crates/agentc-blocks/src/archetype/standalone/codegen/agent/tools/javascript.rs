// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use std::collections::{HashMap, HashSet};

use agentc_compiler::generator::{blocks::codegen::ToIdent, errors::GeneratorError};

use crate::{
    archetype::standalone::{codegen::agent::tools::ToolCodeGen, fields::FieldsSpec},
    context::{ResolvedContext, ResolvedContextToolJavascript, ResolvedContextToolKind},
};

/// All JavaScript tools in the context. Tools that share a bundle path share a single
/// `QuickJsRuntime`, mirroring how embedded Python tools share a runtime per venv.
pub struct JavascriptTools<'a>(pub &'a ResolvedContext);

impl ToolCodeGen for JavascriptTools<'_> {
    fn imports(&self) -> Option<TokenStream> {
        self.0
            .tools
            .values()
            .any(|t| t.kind.is_javascript())
            .then(|| {
                quote! {
                    use agentc_tools::javascript::{QuickJsRuntime, JavascriptTool};
                }
            })
    }

    fn feature(&self) -> Option<&'static str> {
        self.0
            .tools
            .values()
            .any(|t| t.kind.is_javascript())
            .then_some("javascript")
    }

    /// Emits one `QuickJsRuntime` binding per unique bundle path, then one
    /// `.with_tool(JavascriptTool::builder()...)` registration per JS tool.
    fn registrations(&self, fields: &FieldsSpec) -> Result<Vec<TokenStream>, GeneratorError> {
        let ctx = self.0;
        let mut registrations = Vec::new();

        // Group tools by bundle path so each unique bundle shares one runtime.
        let mut by_bundle =
            HashMap::<&str, Vec<(&str, &ResolvedContextToolJavascript)>>::new();
        for (tool_name, tool) in &ctx.tools {
            if let ResolvedContextToolKind::Javascript(js) = &tool.kind {
                by_bundle
                    .entry(js.bundle_path.as_str())
                    .or_default()
                    .push((tool_name.as_str(), js));
            }
        }

        for (bundle_path, tools) in &by_bundle {
            let runtime_ident =
                Ident::new(&format!("js_runtime_{}", bundle_path.to_ident()), Span::call_site());

            // Union of capability strings across all tools sharing this bundle.
            let caps = {
                let mut seen = HashSet::new();
                ctx.tools
                    .iter()
                    .filter(|(_, t)| {
                        matches!(&t.kind, ResolvedContextToolKind::Javascript(js) if js.bundle_path == *bundle_path)
                    })
                    .flat_map(|(_, t)| t.capabilities.iter().map(String::as_str))
                    .filter(|c| seen.insert(*c))
                    .collect::<Vec<_>>()
            };

            let caps_tokens = if caps.is_empty() {
                quote! {}
            } else {
                quote! { .capabilities([#(#caps),*]) }
            };

            registrations.push(quote! {
                #[allow(non_snake_case, nonstandard_style)]
                let #runtime_ident = std::sync::Arc::new(
                    QuickJsRuntime::builder()
                        .source(include_str!(#bundle_path).to_string())
                        #caps_tokens
                        .num_interpreters(4)
                        .shutdown(shutdown.clone())
                        .build()
                        .await?
                );
            });

            for (tool_name, js) in tools {
                let export_name = &js.export_name;

                let tool_caps = ctx
                    .tools
                    .get(*tool_name)
                    .map(|t| {
                        t.capabilities
                            .iter()
                            .map(String::as_str)
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();

                let caps_call = if tool_caps.is_empty() {
                    quote! {}
                } else {
                    quote! { .capabilities([#(#tool_caps),*]) }
                };

                let build_tool = quote! {
                    JavascriptTool::builder()
                        .runtime(#runtime_ident.clone())
                        .export_name(#export_name)
                        #caps_call
                        .build()
                        .await?
                };

                let enabled_path = fields.config_accessor(&["tool", tool_name, "enabled"]);

                if let Some(enabled) = enabled_path {
                    registrations.push(quote! {
                        if #enabled {
                            builder = builder.with_tool(#build_tool);
                        }
                    });
                } else {
                    registrations.push(quote! {
                        builder = builder.with_tool(#build_tool);
                    });
                }
            }
        }

        Ok(registrations)
    }
}
