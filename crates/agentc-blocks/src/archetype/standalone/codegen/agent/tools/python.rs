// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use std::collections::HashMap;

use agentc_compiler::generator::{blocks::codegen::ToIdent, errors::GeneratorError};

use crate::{
    archetype::standalone::{codegen::agent::tools::ToolCodeGen, fields::FieldsSpec},
    context::{
        ResolvedContext, ResolvedContextToolKind, ResolvedContextToolPython,
        ResolvedContextToolPythonInterpreter,
    },
};

/// All embedded-interpreter Python tools in the context. Tools that share a
/// `site_packages_path` share a single `EmbeddedRuntime`.
pub struct EmbeddedPythonTools<'a>(pub &'a ResolvedContext);

impl EmbeddedPythonTools<'_> {
    fn has_embedded(ctx: &ResolvedContext) -> bool {
        ctx.tools.values().any(|t| {
            matches!(
                &t.kind,
                ResolvedContextToolKind::Python(py)
                    if matches!(py.interpreter, ResolvedContextToolPythonInterpreter::Embedded)
            )
        })
    }

    /// Emits the `EmbeddedRuntime::builder()...` binding for a single site-packages group.
    ///
    /// The `site_packages_path` (installed dependencies, including `agentc_tdk`) and each
    /// entry in `project_paths` (the tool package sources) are all frozen into the runtime
    /// so that all tool code and its dependencies are embedded in the binary at compile time.
    fn runtime_binding(
        site_packages_path: &str,
        project_paths: &[&str],
        runtime_ident: &Ident,
    ) -> TokenStream {
        let project_frozen = project_paths.iter().map(|path| {
            quote! {
                .frozen(agentc_tools::python::py_freeze!(dir = #path))
            }
        });

        quote! {
            #[allow(non_snake_case, nonstandard_style)]
            let #runtime_ident = std::sync::Arc::new(
                EmbeddedRuntime::builder()
                    .frozen(agentc_tools::python::py_freeze!(dir = #site_packages_path))
                    #(#project_frozen)*
                    .num_interpreters(4)
                    .channel_size(32)
                    .shutdown(shutdown.clone())
                    .build()?
            );
        }
    }

    /// Emits one `.with_tool(PythonTool::builder()...)` registration per Python tool.
    ///
    /// This helper is backend-agnostic: it takes the resolved `runtime_ident` produced by
    /// whichever runtime generator was called, so the same code runs for both the embedded
    /// and (future) static backends.
    fn tool_registrations(
        tools: &[(&str, &ResolvedContextToolPython)],
        runtime_ident: &Ident,
        ctx: &ResolvedContext,
        fields: &FieldsSpec,
    ) -> Vec<TokenStream> {
        let mut registrations = Vec::new();

        for (tool_name, py) in tools {
            let module_name = py.module_name.as_str();

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
                PythonTool::builder()
                    .runtime(#runtime_ident.clone())
                    .module(#module_name)
                    .tool_name(#tool_name)
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

        registrations
    }
}

impl ToolCodeGen for EmbeddedPythonTools<'_> {
    fn imports(&self) -> Option<TokenStream> {
        Self::has_embedded(self.0).then(|| {
            quote! {
                use agentc_tools::python::{EmbeddedRuntime, PythonTool};
            }
        })
    }

    fn feature(&self) -> Option<&'static str> {
        Self::has_embedded(self.0).then_some("python-embedded")
    }

    fn registrations(&self, fields: &FieldsSpec) -> Result<Vec<TokenStream>, GeneratorError> {
        let ctx = self.0;
        let mut registrations = Vec::new();

        // Group embedded tools by site_packages_path so each unique venv shares one runtime.
        let mut by_site_packages =
            HashMap::<&str, Vec<(&str, &ResolvedContextToolPython)>>::new();
        for (tool_name, tool) in &ctx.tools {
            if let ResolvedContextToolKind::Python(py) = &tool.kind
                && matches!(py.interpreter, ResolvedContextToolPythonInterpreter::Embedded)
            {
                by_site_packages
                    .entry(py.site_packages_path.as_str())
                    .or_default()
                    .push((tool_name.as_str(), py));
            }
        }

        for (site_packages_path, tools) in &by_site_packages {
            let runtime_ident = Ident::new(
                &format!("py_embedded_runtime_{}", site_packages_path.to_ident()),
                Span::call_site(),
            );

            // Collect the distinct project paths for all tools in this group so each
            // tool package's source is also frozen into the shared runtime.
            let mut project_paths = tools
                .iter()
                .map(|(_, py)| py.project_path.as_str())
                .collect::<Vec<_>>();
            project_paths.sort_unstable();
            project_paths.dedup();

            registrations.push(Self::runtime_binding(
                site_packages_path,
                &project_paths,
                &runtime_ident,
            ));
            registrations.extend(Self::tool_registrations(
                tools,
                &runtime_ident,
                ctx,
                fields,
            ));
        }

        Ok(registrations)
    }
}
