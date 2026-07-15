// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use std::collections::HashMap;

use agentc_compiler::generator::{blocks::codegen::ToIdent, errors::GeneratorError};

use crate::{
    context::{
        ResolvedContext, ResolvedContextToolKind, ResolvedContextToolPython,
        ResolvedContextToolPythonInterpreter,
    },
    fields::FieldsSpec,
    graph::codegen::tools::ToolCodeGen,
};

/// Code generation shared by both Python interpreter backends.
struct PythonTools;

impl PythonTools {
    /// Emits one `.with_tool(PythonTool::builder()...)` registration per Python tool.
    ///
    /// Backend-agnostic: it takes the `runtime_ident` bound by whichever backend emitted the
    /// runtime binding, so both backends share one implementation. `PythonTool` is named by
    /// its full path rather than imported, because both backends can appear in the same agent
    /// and each generator's imports land in the same scope.
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
                agentc_tools::python::PythonTool::builder()
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
}

impl ToolCodeGen for EmbeddedPythonTools<'_> {
    fn imports(&self) -> Option<TokenStream> {
        Self::has_embedded(self.0).then(|| {
            quote! {
                use agentc_tools::python::EmbeddedRuntime;
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
        let mut by_site_packages = HashMap::<&str, Vec<(&str, &ResolvedContextToolPython)>>::new();
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
            registrations.extend(PythonTools::tool_registrations(
                tools,
                &runtime_ident,
                ctx,
                fields,
            ));
        }

        Ok(registrations)
    }
}

/// All static-interpreter Python tools in the context. Tools that share a
/// `site_packages_path` share a single `StaticRuntime`.
pub struct StaticPythonTools<'a>(pub &'a ResolvedContext);

impl StaticPythonTools<'_> {
    fn has_static(ctx: &ResolvedContext) -> bool {
        ctx.tools.values().any(|t| {
            matches!(
                &t.kind,
                ResolvedContextToolKind::Python(py)
                    if matches!(py.interpreter, ResolvedContextToolPythonInterpreter::Static)
            )
        })
    }

    /// Emits the `StaticRuntime::builder()...` binding for a single site-packages group.
    ///
    /// The `site_packages_path` (installed dependencies, including `agentc_tdk`) and each
    /// entry in `project_paths` (the tool package sources) are embedded into the binary as
    /// directory trees, so tool code and its dependencies are embedded at compile time just
    /// as they are for the embedded backend. The runtime unpacks them to a temporary
    /// directory and places them on the interpreter's import path.
    fn runtime_binding(
        site_packages_path: &str,
        project_paths: &[&str],
        runtime_ident: &Ident,
    ) -> TokenStream {
        let project_embeds = project_paths.iter().map(|path| {
            quote! {
                .embed(agentc_tools::python::embed_dir!(#path))
            }
        });

        quote! {
            #[allow(non_snake_case, nonstandard_style)]
            let #runtime_ident = std::sync::Arc::new(
                StaticRuntime::builder()
                    .embed(agentc_tools::python::embed_dir!(#site_packages_path))
                    #(#project_embeds)*
                    .num_interpreters(4)
                    .channel_size(32)
                    .shutdown(shutdown.clone())
                    .build()?
            );
        }
    }
}

impl ToolCodeGen for StaticPythonTools<'_> {
    fn imports(&self) -> Option<TokenStream> {
        Self::has_static(self.0).then(|| {
            quote! {
                use agentc_tools::python::StaticRuntime;
            }
        })
    }

    fn feature(&self) -> Option<&'static str> {
        Self::has_static(self.0).then_some("python-static")
    }

    fn registrations(&self, fields: &FieldsSpec) -> Result<Vec<TokenStream>, GeneratorError> {
        let ctx = self.0;
        let mut registrations = Vec::new();

        // Group static tools by site_packages_path so each unique venv shares one runtime.
        let mut by_site_packages = HashMap::<&str, Vec<(&str, &ResolvedContextToolPython)>>::new();
        for (tool_name, tool) in &ctx.tools {
            if let ResolvedContextToolKind::Python(py) = &tool.kind
                && matches!(py.interpreter, ResolvedContextToolPythonInterpreter::Static)
            {
                by_site_packages
                    .entry(py.site_packages_path.as_str())
                    .or_default()
                    .push((tool_name.as_str(), py));
            }
        }

        for (site_packages_path, tools) in &by_site_packages {
            let runtime_ident = Ident::new(
                &format!("py_static_runtime_{}", site_packages_path.to_ident()),
                Span::call_site(),
            );

            // Collect the distinct project paths for all tools in this group so each
            // tool package's source is also embedded into the shared runtime.
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
            registrations.extend(PythonTools::tool_registrations(
                tools,
                &runtime_ident,
                ctx,
                fields,
            ));
        }

        Ok(registrations)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::{
        context::{
            ResolvedContextAgent, ResolvedContextAgentModel, ResolvedContextRuntime,
            ResolvedContextTool,
        },
        graph::codegen::tools::ToolsCodeGen,
        types::RuntimeValue,
    };

    struct PythonToolsFixture;

    impl PythonToolsFixture {
        fn tool(
            name: &str,
            interpreter: ResolvedContextToolPythonInterpreter,
        ) -> (String, ResolvedContextTool) {
            (
                name.to_string(),
                ResolvedContextTool {
                    name: name.to_string(),
                    description: None,
                    enabled: RuntimeValue::constant(true),
                    capabilities: vec![],
                    config: HashMap::new(),
                    kind: ResolvedContextToolKind::Python(ResolvedContextToolPython {
                        project_path: format!("/artifacts/{name}"),
                        site_packages_path: format!(
                            "/artifacts/{name}/.venv/site-packages"
                        ),
                        module_name: name.to_string(),
                        interpreter,
                    }),
                },
            )
        }

        fn context(
            tools: impl IntoIterator<Item = (String, ResolvedContextTool)>,
        ) -> ResolvedContext {
            ResolvedContext {
                slug: "assistant".to_string(),
                agent_name: "assistant".to_string(),
                runtime: ResolvedContextRuntime {
                    default_tenant_id: RuntimeValue::constant("default".to_string()),
                },
                providers: vec![],
                agent: ResolvedContextAgent {
                    version: "0.1.0".to_string(),
                    description: None,
                    prompt: None,
                    capabilities: None,
                    capability_policy: None,
                    model: ResolvedContextAgentModel {
                        provider: RuntimeValue::constant("anthropic".to_string()),
                        name: RuntimeValue::constant("claude".to_string()),
                    },
                },
                blocks: HashMap::new(),
                tools: tools.into_iter().collect(),
                skills: HashMap::new(),
                http_server: None,
            }
        }

        fn generated(ctx: &ResolvedContext) -> (String, String) {
            let (imports, registrations) =
                ToolsCodeGen::generate(ctx, &FieldsSpec::collect_from(ctx))
                    .expect("tool code generation should succeed");

            (
                imports
                    .into_iter()
                    .map(|tokens| tokens.to_string())
                    .collect::<Vec<_>>()
                    .join("\n"),
                registrations
                    .into_iter()
                    .map(|tokens| tokens.to_string())
                    .collect::<Vec<_>>()
                    .join("\n"),
            )
        }
    }

    #[test]
    fn static_interpreter_generates_static_runtime_binding() {
        let ctx = PythonToolsFixture::context([PythonToolsFixture::tool(
            "static_adder",
            ResolvedContextToolPythonInterpreter::Static,
        )]);
        let (imports, registrations) = PythonToolsFixture::generated(&ctx);

        assert!(imports.contains("StaticRuntime"));
        assert!(registrations.contains("StaticRuntime :: builder"));
        assert!(registrations.contains("embed_dir !"));
        assert!(registrations.contains("PythonTool :: builder"));
        assert!(registrations.contains("static_adder"));
        assert!(
            ToolsCodeGen::features(&ctx)
                .to_string()
                .contains("python-static")
        );
    }

    #[test]
    fn embedded_interpreter_generates_embedded_runtime_binding() {
        let ctx = PythonToolsFixture::context([PythonToolsFixture::tool(
            "embedded_adder",
            ResolvedContextToolPythonInterpreter::Embedded,
        )]);
        let (imports, registrations) = PythonToolsFixture::generated(&ctx);

        assert!(imports.contains("EmbeddedRuntime"));
        assert!(registrations.contains("EmbeddedRuntime :: builder"));
        assert!(registrations.contains("py_freeze !"));
        assert!(registrations.contains("PythonTool :: builder"));
        assert!(registrations.contains("embedded_adder"));
        assert!(
            ToolsCodeGen::features(&ctx)
                .to_string()
                .contains("python-embedded")
        );
    }

    #[test]
    fn mixed_interpreters_generate_both_backends() {
        let ctx = PythonToolsFixture::context([
            PythonToolsFixture::tool(
                "embedded_adder",
                ResolvedContextToolPythonInterpreter::Embedded,
            ),
            PythonToolsFixture::tool(
                "static_adder",
                ResolvedContextToolPythonInterpreter::Static,
            ),
        ]);
        let (imports, registrations) = PythonToolsFixture::generated(&ctx);
        let features = ToolsCodeGen::features(&ctx).to_string();

        assert!(imports.contains("EmbeddedRuntime"));
        assert!(imports.contains("StaticRuntime"));
        assert_eq!(imports.matches("PythonTool").count(), 0);
        assert!(registrations.contains("EmbeddedRuntime :: builder"));
        assert!(registrations.contains("StaticRuntime :: builder"));
        assert_eq!(registrations.matches("PythonTool :: builder").count(), 2);
        assert!(registrations.contains("embedded_adder"));
        assert!(registrations.contains("static_adder"));
        assert!(features.contains("python-embedded"));
        assert!(features.contains("python-static"));
    }
}
