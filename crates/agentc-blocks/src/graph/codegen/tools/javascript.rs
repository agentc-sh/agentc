// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use std::collections::HashMap;

use agentc_compiler::generator::{
    blocks::{codegen::ToIdent, fragment::Fragment},
    context::GenerationContext,
    errors::GeneratorError,
    extension::ErasedContributionValue,
};

use crate::{
    config::fields::FieldsSpec,
    context::{
        ResolvedContext, ResolvedContextTool, ResolvedContextToolJavascript,
        ResolvedContextToolKind,
    },
    contributions::{
        dependency::{
            CargoDependencies, CargoDependencyContribution, CargoPatchContribution, CargoPatches,
            RuntimeDependencyContribution,
        },
        import::ImportContribution,
    },
    graph::codegen::tools::{ToolCodeGen, enabled::ToolEnabledGuard},
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
    fn imports(&self) -> Vec<ImportContribution> {
        Self::is_present(self.0)
            .then(|| {
                vec![
                    ImportContribution::path(&["agentc_executor_typescript", "executor"])
                        .item("Executor"),
                    ImportContribution::path(&["agentc_fs", "typescript", "executor"])
                        .item("ExecutorBuilderFsExt"),
                    ImportContribution::path(&["agentc_http", "client", "typescript"])
                        .item("ExecutorBuilderHttpExt"),
                    ImportContribution::path(&["agentc_tools", "javascript"])
                        .item("ExecutorBuilderToolsExt")
                        .item("JavascriptTool"),
                ]
            })
            .unwrap_or_default()
    }

    fn feature(&self) -> Option<&'static str> {
        Self::is_present(self.0).then_some("javascript")
    }

    /// Emits one `Executor` binding per unique bundle path, then one
    /// `.with_tool(JavascriptTool::builder()...)` registration per JS tool.
    fn registrations(&self, fields: &FieldsSpec) -> Result<Vec<TokenStream>, GeneratorError> {
        let mut registrations = Vec::new();

        let mut by_bundle =
            HashMap::<&str, Vec<(&ResolvedContextTool, &ResolvedContextToolJavascript)>>::new();

        for tool in self.0.tools.values() {
            if let ResolvedContextToolKind::Javascript(js) = &tool.kind {
                by_bundle
                    .entry(js.bundle_path.as_str())
                    .or_default()
                    .push((tool, js));
            }
        }

        for (bundle_path, tools) in &by_bundle {
            let executor_ident =
                Ident::new(&format!("js_executor_{}", bundle_path.to_ident()), Span::call_site());

            registrations.push(quote! {
                #[allow(non_snake_case, nonstandard_style)]
                let #executor_ident = Executor::builder(#bundle_path, include_str!(#bundle_path))
                    .workers(4)
                    .queue_capacity(32)
                    .standard_environment()
                    .with_tools()
                    .with_http(config.network.builder()?)
                    .with_fs(fs.root())?
                    .cancellation(shutdown.clone())
                    .build()
                    .await?;
            });

            for (tool, js) in tools {
                let export_name = &js.export_name;
                let name = &tool.name;
                let description = tool
                    .description
                    .as_ref()
                    .map(|description| quote! { .description(#description) });
                let capabilities = (!tool.capabilities.is_empty()).then(|| {
                    let capabilities = &tool.capabilities;

                    quote! { .capabilities([#(#capabilities),*]) }
                });

                registrations.push(ToolEnabledGuard(tool).wrap(
                    fields,
                    quote! {
                        builder = builder.with_tool(
                            JavascriptTool::builder()
                                .executor(#executor_ident.clone())
                                .export_name(#export_name)
                                .name(#name)
                                #description
                                #capabilities
                                .build()
                                .await?
                        );
                    },
                ));
            }
        }

        Ok(registrations)
    }
}

pub struct JavascriptToolCargoFragment;

impl Fragment<ResolvedContext> for JavascriptToolCargoFragment {
    fn generate_contribution(
        &self,
        _ctx: &GenerationContext<ResolvedContext>,
        point: &str,
    ) -> Result<ErasedContributionValue, GeneratorError> {
        match point {
            "cargo::dependencies" => Ok(ErasedContributionValue::new(
                CargoDependencies::from_entries([CargoDependencyContribution::runtime(
                    RuntimeDependencyContribution::new("agentc-executor-typescript"),
                )])
                .map_err(|error| GeneratorError::unexpected(error.to_string()))?,
            )),
            "cargo::patches" => Ok(ErasedContributionValue::new(
                CargoPatches::from_entries([CargoPatchContribution::runtime(
                    RuntimeDependencyContribution::new("agentc-executor-typescript"),
                )])
                .map_err(|error| GeneratorError::unexpected(error.to_string()))?,
            )),
            _ => Err(GeneratorError::unexpected(format!("Unknown extension point '{}'", point))),
        }
    }
}

pub struct HttpTypescriptCargoFragment;

impl Fragment<ResolvedContext> for HttpTypescriptCargoFragment {
    fn generate_contribution(
        &self,
        _ctx: &GenerationContext<ResolvedContext>,
        point: &str,
    ) -> Result<ErasedContributionValue, GeneratorError> {
        match point {
            "cargo::dependencies" => Ok(ErasedContributionValue::new(
                CargoDependencies::from_entries([CargoDependencyContribution::runtime(
                    RuntimeDependencyContribution::new("agentc-http")
                        .default_features(false)
                        .feature("typescript"),
                )])
                .map_err(|error| GeneratorError::unexpected(error.to_string()))?,
            )),
            _ => Err(GeneratorError::unexpected(format!("Unknown extension point '{}'", point))),
        }
    }
}

pub struct FilesystemTypescriptCargoFragment;

impl Fragment<ResolvedContext> for FilesystemTypescriptCargoFragment {
    fn generate_contribution(
        &self,
        _ctx: &GenerationContext<ResolvedContext>,
        point: &str,
    ) -> Result<ErasedContributionValue, GeneratorError> {
        match point {
            "cargo::dependencies" => Ok(ErasedContributionValue::new(
                CargoDependencies::from_entries([CargoDependencyContribution::runtime(
                    RuntimeDependencyContribution::new("agentc-fs")
                        .default_features(false)
                        .feature("typescript"),
                )])
                .map_err(|error| GeneratorError::unexpected(error.to_string()))?,
            )),
            _ => Err(GeneratorError::unexpected(format!("Unknown extension point '{}'", point))),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::{
        context::{
            ResolvedContextAgent, ResolvedContextAgentModel, ResolvedContextFilesystem,
            ResolvedContextNetwork, ResolvedContextRuntime, ResolvedContextTool,
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
                network: ResolvedContextNetwork::default(),
                filesystem: ResolvedContextFilesystem::default(),
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
        assert!(registrations.contains(". with_tools ()"));
        assert!(registrations.contains(". with_http (config . network . builder () ?)"));
        assert!(registrations.contains(". with_fs (fs . root ())"));
        assert!(registrations.contains(". cancellation (shutdown . clone ())"));
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
    fn registration_passes_the_block_name() {
        let ctx = JavascriptToolsFixture::context([JavascriptToolsFixture::tool(
            "search_document_sections",
            "/artifacts/pkg/dist/index.js",
            "SearchDocumentSections",
            [],
        )]);
        let registrations = JavascriptToolsFixture::registrations(&ctx);

        assert!(registrations.contains(". export_name (\"SearchDocumentSections\")"));
        assert!(registrations.contains(". name (\"search_document_sections\")"));
        assert!(!registrations.contains(". description ("));
        assert!(!registrations.contains(". capabilities ("));
    }

    #[test]
    fn registration_passes_the_block_description_when_set() {
        let (name, mut tool) = JavascriptToolsFixture::tool(
            "search",
            "/artifacts/pkg/dist/index.js",
            "Search",
            [],
        );
        tool.description = Some("Searches the documentation.".to_string());

        let registrations =
            JavascriptToolsFixture::registrations(&JavascriptToolsFixture::context([(name, tool)]));

        assert!(registrations.contains(". description (\"Searches the documentation.\")"));
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
        assert_eq!(
            JavascriptTools(&ctx).imports(),
            vec![
                ImportContribution::path(&["agentc_executor_typescript", "executor"])
                    .item("Executor"),
                ImportContribution::path(&["agentc_fs", "typescript", "executor"])
                    .item("ExecutorBuilderFsExt"),
                ImportContribution::path(&["agentc_http", "client", "typescript"])
                    .item("ExecutorBuilderHttpExt"),
                ImportContribution::path(&["agentc_tools", "javascript"])
                    .item("ExecutorBuilderToolsExt")
                    .item("JavascriptTool"),
            ],
        );
    }

    #[test]
    fn absent_javascript_tools_generate_no_imports_or_registrations() {
        let ctx = JavascriptToolsFixture::context([]);

        assert!(
            JavascriptTools(&ctx)
                .imports()
                .is_empty()
        );
        assert!(
            JavascriptTools(&ctx)
                .feature()
                .is_none()
        );
        assert!(JavascriptToolsFixture::registrations(&ctx).is_empty());
    }
}
