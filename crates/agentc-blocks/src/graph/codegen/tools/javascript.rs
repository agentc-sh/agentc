// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use convert_case::{Case, Casing};
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use agentc_compiler::generator::{
    blocks::{codegen::ToIdent, template::TemplateFragment},
    context::GenerationContext,
    errors::GeneratorError,
    extension::{ErasedContributionValue, ExtensionRegistry},
};

use crate::{
    context::{ResolvedContext, ResolvedContextToolJavascript, ResolvedContextToolKind},
    contributions::dependency::{
        CargoDependencyContribution, CargoPatchContribution, RuntimeDependencyContribution,
    },
    fields::FieldsSpec,
    graph::codegen::tools::ToolCodeGen,
};

/// All JavaScript tools in the context. Tools that share a bundle path share a single
/// `Executor`, mirroring how embedded Python tools share a runtime per venv.
pub struct JavascriptTools<'a>(pub &'a ResolvedContext);

impl JavascriptTools<'_> {
    pub fn is_present(ctx: &ResolvedContext) -> bool {
        ctx.tools
            .values()
            .any(|tool| tool.kind.is_javascript())
    }
}

impl ToolCodeGen for JavascriptTools<'_> {
    fn imports(&self) -> Option<TokenStream> {
        Self::is_present(self.0).then(|| {
            quote! {
                use agentc_executor_typescript::executor::Executor;
                use agentc_tools::javascript::{ExecutorBuilderToolExt, JavascriptTool};
            }
        })
    }

    fn feature(&self) -> Option<&'static str> {
        Self::is_present(self.0).then_some("javascript")
    }

    /// Emits one `Executor` binding per unique bundle path, then one
    /// `.with_tool(JavascriptTool::builder()...)` registration per JS tool.
    fn registrations(&self, fields: &FieldsSpec) -> Result<Vec<TokenStream>, GeneratorError> {
        let ctx = self.0;
        let mut registrations = Vec::new();

        // Group tools by bundle path so each unique bundle shares one executor.
        let mut by_bundle = HashMap::<&str, Vec<(&str, &ResolvedContextToolJavascript)>>::new();
        for (tool_name, tool) in &ctx.tools {
            if let ResolvedContextToolKind::Javascript(js) = &tool.kind {
                by_bundle
                    .entry(js.bundle_path.as_str())
                    .or_default()
                    .push((tool_name.as_str(), js));
            }
        }

        for (bundle_path, tools) in &by_bundle {
            let executor_ident =
                Ident::new(&format!("js_executor_{}", bundle_path.to_ident()), Span::call_site());

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

            let caps_call = if caps.is_empty() {
                quote! {}
            } else {
                quote! { .with_tool_capabilities([#(#caps),*]) }
            };

            registrations.push(quote! {
                #[allow(non_snake_case, nonstandard_style)]
                let #executor_ident = Executor::builder(#bundle_path, include_str!(#bundle_path))
                    .workers(4)
                    .queue_capacity(32)
                    .standard_environment()
                    #caps_call
                    .cancellation(shutdown.clone())
                    .build()
                    .await?;
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
                        .executor(#executor_ident.clone())
                        .export_name(#export_name)
                        #caps_call
                        .build()
                        .await?
                };

                let enabled_path = fields.config_accessor(&[
                    "tool",
                    &if tool_name.contains(|c: char| !c.is_alphanumeric() && c != '_') {
                        tool_name.to_case(Case::Snake)
                    } else {
                        tool_name.to_string()
                    },
                    "enabled",
                ]);

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

pub struct JavascriptToolCargoFragment;

impl TemplateFragment<ResolvedContext> for JavascriptToolCargoFragment {
    fn generate_contribution(
        &self,
        _ctx: &GenerationContext<ResolvedContext>,
        point: &str,
    ) -> Result<ErasedContributionValue, GeneratorError> {
        match point {
            "cargo::dependencies" => {
                Ok(ErasedContributionValue::new(CargoDependencyContribution::runtime(
                    RuntimeDependencyContribution::new("agentc-executor-typescript"),
                )))
            }
            "cargo::patches" => Ok(ErasedContributionValue::new(CargoPatchContribution::runtime(
                RuntimeDependencyContribution::new("agentc-executor-typescript"),
            ))),
            _ => Err(GeneratorError::unexpected(format!("Unknown extension point '{}'", point))),
        }
    }

    fn generate_files(
        &self,
        _ctx: &GenerationContext<ResolvedContext>,
        _registry: &ExtensionRegistry,
    ) -> Result<Vec<(PathBuf, String)>, GeneratorError> {
        Ok(vec![])
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
        types::RuntimeValue,
    };

    struct JavascriptToolsFixture;

    impl JavascriptToolsFixture {
        fn tool(
            name: &str,
            bundle_path: &str,
            export_name: &str,
            capabilities: impl IntoIterator<Item = &'static str>,
        ) -> (String, ResolvedContextTool) {
            (
                name.to_string(),
                ResolvedContextTool {
                    name: name.to_string(),
                    description: None,
                    enabled: RuntimeValue::constant(true),
                    capabilities: capabilities
                        .into_iter()
                        .map(String::from)
                        .collect(),
                    config: HashMap::new(),
                    kind: ResolvedContextToolKind::Javascript(ResolvedContextToolJavascript {
                        bundle_path: bundle_path.to_string(),
                        export_name: export_name.to_string(),
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

        fn registrations(ctx: &ResolvedContext) -> String {
            JavascriptTools(ctx)
                .registrations(&FieldsSpec::collect_from(ctx))
                .expect("javascript registrations should succeed")
                .into_iter()
                .map(|tokens| tokens.to_string())
                .collect::<Vec<_>>()
                .join("\n")
        }
    }

    #[test]
    fn shared_bundle_generates_one_executor_for_all_exports() {
        let ctx = JavascriptToolsFixture::context([
            JavascriptToolsFixture::tool("search", "/artifacts/pkg/dist/index.js", "search", []),
            JavascriptToolsFixture::tool("lookup", "/artifacts/pkg/dist/index.js", "lookup", []),
        ]);
        let registrations = JavascriptToolsFixture::registrations(&ctx);

        assert_eq!(
            registrations
                .matches("Executor :: builder")
                .count(),
            1
        );
        assert_eq!(
            registrations
                .matches("JavascriptTool :: builder")
                .count(),
            2
        );
        assert!(registrations.contains("include_str ! (\"/artifacts/pkg/dist/index.js\")"));
        assert!(registrations.contains("\"search\""));
        assert!(registrations.contains("\"lookup\""));
    }

    #[test]
    fn shared_bundle_clones_one_executor_into_each_tool() {
        let ctx = JavascriptToolsFixture::context([
            JavascriptToolsFixture::tool("search", "/artifacts/pkg/dist/index.js", "search", []),
            JavascriptToolsFixture::tool("lookup", "/artifacts/pkg/dist/index.js", "lookup", []),
        ]);
        let registrations = JavascriptToolsFixture::registrations(&ctx);

        assert_eq!(
            registrations
                .matches(". executor (js_executor_")
                .count(),
            2
        );
    }

    #[test]
    fn separate_bundles_generate_separate_executors() {
        let ctx = JavascriptToolsFixture::context([
            JavascriptToolsFixture::tool("a", "/artifacts/a/dist/index.js", "a", []),
            JavascriptToolsFixture::tool("b", "/artifacts/b/dist/index.js", "b", []),
        ]);
        let registrations = JavascriptToolsFixture::registrations(&ctx);

        assert_eq!(
            registrations
                .matches("Executor :: builder")
                .count(),
            2
        );
        assert!(registrations.contains("include_str ! (\"/artifacts/a/dist/index.js\")"));
        assert!(registrations.contains("include_str ! (\"/artifacts/b/dist/index.js\")"));
    }

    #[test]
    fn executor_uses_four_workers_queue_and_shutdown_cancellation() {
        let ctx = JavascriptToolsFixture::context([JavascriptToolsFixture::tool(
            "search",
            "/artifacts/pkg/dist/index.js",
            "search",
            [],
        )]);
        let registrations = JavascriptToolsFixture::registrations(&ctx);

        assert!(registrations.contains(". workers (4)"));
        assert!(registrations.contains(". queue_capacity (32)"));
        assert!(registrations.contains(". standard_environment ()"));
        assert!(registrations.contains(". cancellation (shutdown . clone ())"));
    }

    #[test]
    fn package_capabilities_are_unioned_across_shared_exports() {
        let ctx = JavascriptToolsFixture::context([
            JavascriptToolsFixture::tool(
                "search",
                "/artifacts/pkg/dist/index.js",
                "search",
                ["network"],
            ),
            JavascriptToolsFixture::tool(
                "read",
                "/artifacts/pkg/dist/index.js",
                "read",
                ["filesystem::read"],
            ),
        ]);
        let registrations = JavascriptToolsFixture::registrations(&ctx);

        assert_eq!(
            registrations
                .matches(". with_tool_capabilities (")
                .count(),
            1
        );
        assert!(registrations.contains("\"network\""));
        assert!(registrations.contains("\"filesystem::read\""));
    }

    #[test]
    fn each_tool_reports_only_its_own_capabilities() {
        let ctx = JavascriptToolsFixture::context([JavascriptToolsFixture::tool(
            "search",
            "/artifacts/pkg/dist/index.js",
            "search",
            ["network"],
        )]);
        let registrations = JavascriptToolsFixture::registrations(&ctx);

        assert!(registrations.contains(". capabilities ([\"network\"])"));
    }

    #[test]
    fn registration_is_guarded_by_the_generated_enabled_field() {
        let ctx = JavascriptToolsFixture::context([JavascriptToolsFixture::tool(
            "search",
            "/artifacts/pkg/dist/index.js",
            "search",
            [],
        )]);
        let registrations = JavascriptToolsFixture::registrations(&ctx);

        assert!(registrations.contains("if config . tool . search . enabled"));
        assert!(registrations.contains("builder = builder . with_tool"));
    }

    #[test]
    fn imports_reference_the_executor_and_tool_surface() {
        let ctx = JavascriptToolsFixture::context([JavascriptToolsFixture::tool(
            "search",
            "/artifacts/pkg/dist/index.js",
            "search",
            [],
        )]);
        let imports = JavascriptTools(&ctx)
            .imports()
            .expect("javascript imports are present")
            .to_string();

        assert!(imports.contains("agentc_executor_typescript :: executor :: Executor"));
        assert!(imports.contains("ExecutorBuilderToolExt"));
        assert!(imports.contains("JavascriptTool"));
    }

    #[test]
    fn absent_javascript_tools_generate_no_imports_or_registrations() {
        let ctx = JavascriptToolsFixture::context([]);

        assert!(
            JavascriptTools(&ctx)
                .imports()
                .is_none()
        );
        assert!(
            JavascriptTools(&ctx)
                .feature()
                .is_none()
        );
        assert!(JavascriptToolsFixture::registrations(&ctx).is_empty());
    }
}
