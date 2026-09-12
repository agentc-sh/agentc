// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use convert_case::{Case, Casing};
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
        ResolvedContext, ResolvedContextToolKind, ResolvedContextToolPython,
        ResolvedContextToolPythonInterpreter,
    },
    contributions::{
        dependency::{
            CargoDependencies, CargoDependencyContribution, CargoPatchContribution, CargoPatches,
            RuntimeDependencyContribution,
        },
        import::ImportContribution,
    },
    graph::codegen::tools::ToolCodeGen,
};

trait PythonBackend {
    const FEATURE: &'static str;
    const EXECUTOR_FEATURE: &'static str;
    const IDENT_PREFIX: &'static str;

    fn selects(interpreter: &ResolvedContextToolPythonInterpreter) -> bool;

    fn backend_type() -> TokenStream;
}

struct PythonTools;

impl PythonTools {
    fn by_package<B: PythonBackend>(
        ctx: &ResolvedContext,
    ) -> HashMap<&str, Vec<(&str, &ResolvedContextToolPython)>> {
        let mut by_package = HashMap::<&str, Vec<(&str, &ResolvedContextToolPython)>>::new();

        for (tool_name, tool) in &ctx.tools {
            if let ResolvedContextToolKind::Python(py) = &tool.kind
                && B::selects(&py.interpreter)
            {
                by_package
                    .entry(py.project_path.as_str())
                    .or_default()
                    .push((tool_name.as_str(), py));
            }
        }

        by_package
    }

    fn executor_binding<B: PythonBackend>(
        py: &ResolvedContextToolPython,
        executor_ident: &Ident,
    ) -> TokenStream {
        let backend = B::backend_type();
        let module_name = py.module_name.as_str();
        let project_path = py.project_path.as_str();
        let site_packages_path = py.site_packages_path.as_str();

        quote! {
            #[allow(non_snake_case, nonstandard_style)]
            let #executor_ident =
                agentc_executor_python::executor::Executor::<#backend>::builder(#module_name)
                    .bundle(agentc_executor_python::bundle!(#project_path)?)
                    .bundle(agentc_executor_python::bundle!(#site_packages_path)?)
                    .with_tools()
                    .workers(4)
                    .queue_capacity(32)
                    .cancellation(shutdown.clone())
                    .build()
                    .await?;
        }
    }

    fn tool_registrations(
        tools: &[(&str, &ResolvedContextToolPython)],
        executor_ident: &Ident,
        ctx: &ResolvedContext,
        fields: &FieldsSpec,
    ) -> Vec<TokenStream> {
        let mut registrations = Vec::new();

        for (tool_name, py) in tools {
            let export_name = &py.export_name;
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

        registrations
    }

    fn is_present<B: PythonBackend>(ctx: &ResolvedContext) -> bool {
        ctx.tools.values().any(|tool| {
            matches!(
                &tool.kind,
                ResolvedContextToolKind::Python(py) if B::selects(&py.interpreter)
            )
        })
    }

    fn imports<B: PythonBackend>(ctx: &ResolvedContext) -> Vec<ImportContribution> {
        Self::is_present::<B>(ctx)
            .then(|| {
                vec![
                    ImportContribution::path(&["agentc_tools", "python"])
                        .item_as("ExecutorBuilderToolsExt", "_"),
                ]
            })
            .unwrap_or_default()
    }

    fn registrations<B: PythonBackend>(
        ctx: &ResolvedContext,
        fields: &FieldsSpec,
    ) -> Vec<TokenStream> {
        let mut registrations = Vec::new();

        for (project_path, tools) in &Self::by_package::<B>(ctx) {
            let executor_ident = Ident::new(
                &format!("{}{}", B::IDENT_PREFIX, project_path.to_ident()),
                Span::call_site(),
            );

            registrations.push(Self::executor_binding::<B>(tools[0].1, &executor_ident));
            registrations.extend(Self::tool_registrations(tools, &executor_ident, ctx, fields));
        }

        registrations
    }
}

pub struct RustPythonTools<'a>(pub &'a ResolvedContext);

impl PythonBackend for RustPythonTools<'_> {
    const FEATURE: &'static str = "python-rustpython";
    const EXECUTOR_FEATURE: &'static str = "rustpython";
    const IDENT_PREFIX: &'static str = "py_rustpython_executor_";

    fn selects(interpreter: &ResolvedContextToolPythonInterpreter) -> bool {
        matches!(interpreter, ResolvedContextToolPythonInterpreter::Embedded)
    }

    fn backend_type() -> TokenStream {
        quote! { agentc_executor_python::guestpy::rustpython::RustPython }
    }
}

impl RustPythonTools<'_> {
    pub fn is_present(ctx: &ResolvedContext) -> bool {
        PythonTools::is_present::<Self>(ctx)
    }
}

impl ToolCodeGen for RustPythonTools<'_> {
    fn imports(&self) -> Vec<ImportContribution> {
        PythonTools::imports::<Self>(self.0)
    }

    fn feature(&self) -> Option<&'static str> {
        Self::is_present(self.0).then_some(Self::FEATURE)
    }

    fn registrations(&self, fields: &FieldsSpec) -> Result<Vec<TokenStream>, GeneratorError> {
        Ok(PythonTools::registrations::<Self>(self.0, fields))
    }
}

pub struct CPythonTools<'a>(pub &'a ResolvedContext);

impl PythonBackend for CPythonTools<'_> {
    const FEATURE: &'static str = "python-cpython";
    const EXECUTOR_FEATURE: &'static str = "pyo3";
    const IDENT_PREFIX: &'static str = "py_cpython_executor_";

    fn selects(interpreter: &ResolvedContextToolPythonInterpreter) -> bool {
        matches!(interpreter, ResolvedContextToolPythonInterpreter::Static)
    }

    fn backend_type() -> TokenStream {
        quote! { agentc_executor_python::guestpy::pyo3::CPython }
    }
}

impl CPythonTools<'_> {
    pub fn is_present(ctx: &ResolvedContext) -> bool {
        PythonTools::is_present::<Self>(ctx)
    }
}

impl ToolCodeGen for CPythonTools<'_> {
    fn imports(&self) -> Vec<ImportContribution> {
        PythonTools::imports::<Self>(self.0)
    }

    fn feature(&self) -> Option<&'static str> {
        Self::is_present(self.0).then_some(Self::FEATURE)
    }

    fn registrations(&self, fields: &FieldsSpec) -> Result<Vec<TokenStream>, GeneratorError> {
        Ok(PythonTools::registrations::<Self>(self.0, fields))
    }
}

pub struct PythonToolCargoFragment;

impl PythonToolCargoFragment {
    fn dependency(ctx: &ResolvedContext) -> RuntimeDependencyContribution {
        let mut dependency =
            RuntimeDependencyContribution::new("agentc-executor-python").default_features(false);

        if RustPythonTools::is_present(ctx) {
            dependency = dependency.feature(RustPythonTools::EXECUTOR_FEATURE);
        }

        if CPythonTools::is_present(ctx) {
            dependency = dependency.feature(CPythonTools::EXECUTOR_FEATURE);
        }

        dependency
    }
}

impl Fragment<ResolvedContext> for PythonToolCargoFragment {
    fn generate_contribution(
        &self,
        ctx: &GenerationContext<ResolvedContext>,
        point: &str,
    ) -> Result<ErasedContributionValue, GeneratorError> {
        match point {
            "cargo::dependencies" => Ok(ErasedContributionValue::new(
                CargoDependencies::from_entries([CargoDependencyContribution::runtime(
                    Self::dependency(ctx.as_inner()),
                )])
                .map_err(|error| GeneratorError::unexpected(error.to_string()))?,
            )),
            "cargo::patches" => Ok(ErasedContributionValue::new(
                CargoPatches::from_entries([CargoPatchContribution::runtime(
                    RuntimeDependencyContribution::new("agentc-executor-python"),
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

    use agentc_compiler::generator::extension::ExtensionPoint;

    use super::*;
    use crate::{
        context::{
            ResolvedContextAgent, ResolvedContextAgentModel, ResolvedContextFilesystem,
            ResolvedContextNetwork, ResolvedContextRuntime, ResolvedContextTool,
        },
        contributions::import::ImportsExtensionPoint,
        graph::codegen::tools::ToolsCodeGen,
        types::RuntimeValue,
    };

    struct PythonToolsFixture;

    impl PythonToolsFixture {
        fn tool(
            name: &str,
            project_path: &str,
            export_name: &str,
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
                        project_path: project_path.to_string(),
                        site_packages_path: format!("{project_path}/.venv/site-packages"),
                        module_name: project_path
                            .rsplit('/')
                            .next()
                            .expect("project path has a final segment")
                            .to_string(),
                        export_name: export_name.to_string(),
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
                network: ResolvedContextNetwork::default(),
                filesystem: ResolvedContextFilesystem::default(),
            }
        }

        fn generated(ctx: &ResolvedContext) -> (String, String) {
            (
                ExtensionPoint::reduce(
                    &ImportsExtensionPoint::new("agent::use"),
                    vec![ToolsCodeGen::imports(ctx).expect("tool imports should merge")],
                )
                .expect("tool imports should render"),
                ToolsCodeGen::registrations(ctx, &FieldsSpec::collect_from(ctx))
                    .expect("tool code generation should succeed")
                    .into_iter()
                    .map(|tokens| tokens.to_string())
                    .collect::<Vec<_>>()
                    .join("\n"),
            )
        }
    }

    #[test]
    fn rustpython_interpreter_generates_a_rustpython_executor() {
        let ctx = PythonToolsFixture::context([PythonToolsFixture::tool(
            "adder",
            "/artifacts/adder",
            "Adder",
            ResolvedContextToolPythonInterpreter::Embedded,
        )]);
        let (_, registrations) = PythonToolsFixture::generated(&ctx);

        assert!(registrations.contains("agentc_executor_python :: executor :: Executor"));
        assert!(registrations.contains("guestpy :: rustpython :: RustPython"));
        assert_eq!(
            registrations
                .matches("bundle !")
                .count(),
            2
        );
        assert!(registrations.contains("PythonTool :: builder"));
        assert!(registrations.contains("adder"));
        assert!(
            ToolsCodeGen::features(&ctx)
                .to_string()
                .contains("python-rustpython")
        );
    }

    #[test]
    fn cpython_interpreter_generates_a_cpython_executor() {
        let ctx = PythonToolsFixture::context([PythonToolsFixture::tool(
            "adder",
            "/artifacts/adder",
            "Adder",
            ResolvedContextToolPythonInterpreter::Static,
        )]);
        let (_, registrations) = PythonToolsFixture::generated(&ctx);

        assert!(registrations.contains("agentc_executor_python :: executor :: Executor"));
        assert!(registrations.contains("guestpy :: pyo3 :: CPython"));
        assert_eq!(
            registrations
                .matches("bundle !")
                .count(),
            2
        );
        assert!(registrations.contains("PythonTool :: builder"));
        assert!(registrations.contains("adder"));
        assert!(
            ToolsCodeGen::features(&ctx)
                .to_string()
                .contains("python-cpython")
        );
    }

    #[test]
    fn mixed_interpreters_generate_both_backends() {
        let ctx = PythonToolsFixture::context([
            PythonToolsFixture::tool(
                "embedded_adder",
                "/artifacts/embedded_adder",
                "EmbeddedAdder",
                ResolvedContextToolPythonInterpreter::Embedded,
            ),
            PythonToolsFixture::tool(
                "static_adder",
                "/artifacts/static_adder",
                "StaticAdder",
                ResolvedContextToolPythonInterpreter::Static,
            ),
        ]);
        let (imports, registrations) = PythonToolsFixture::generated(&ctx);
        let features = ToolsCodeGen::features(&ctx).to_string();

        assert_eq!(imports, "use agentc_tools::python::ExecutorBuilderToolsExt as _;");
        assert!(registrations.contains("guestpy :: rustpython :: RustPython"));
        assert!(registrations.contains("guestpy :: pyo3 :: CPython"));
        assert_eq!(
            registrations
                .matches("PythonTool :: builder")
                .count(),
            2
        );
        assert!(registrations.contains("embedded_adder"));
        assert!(registrations.contains("static_adder"));
        assert!(features.contains("python-rustpython"));
        assert!(features.contains("python-cpython"));
    }

    #[test]
    fn shared_package_generates_one_executor_for_all_tools() {
        let ctx = PythonToolsFixture::context([
            PythonToolsFixture::tool(
                "adder",
                "/artifacts/mathkit",
                "Adder",
                ResolvedContextToolPythonInterpreter::Embedded,
            ),
            PythonToolsFixture::tool(
                "subtractor",
                "/artifacts/mathkit",
                "Subtractor",
                ResolvedContextToolPythonInterpreter::Embedded,
            ),
        ]);
        let (_, registrations) = PythonToolsFixture::generated(&ctx);

        assert_eq!(
            registrations
                .matches("Executor :: < agentc_executor_python")
                .count(),
            1
        );
        assert_eq!(
            registrations
                .matches("PythonTool :: builder")
                .count(),
            2
        );
    }

    #[test]
    fn shared_package_clones_one_executor_into_each_tool() {
        let ctx = PythonToolsFixture::context([
            PythonToolsFixture::tool(
                "adder",
                "/artifacts/mathkit",
                "Adder",
                ResolvedContextToolPythonInterpreter::Embedded,
            ),
            PythonToolsFixture::tool(
                "subtractor",
                "/artifacts/mathkit",
                "Subtractor",
                ResolvedContextToolPythonInterpreter::Embedded,
            ),
        ]);
        let (_, registrations) = PythonToolsFixture::generated(&ctx);

        assert_eq!(
            registrations
                .matches(". executor (py_rustpython_executor_")
                .count(),
            2
        );
    }

    #[test]
    fn separate_packages_generate_separate_executors() {
        let ctx = PythonToolsFixture::context([
            PythonToolsFixture::tool(
                "adder",
                "/artifacts/mathkit",
                "Adder",
                ResolvedContextToolPythonInterpreter::Embedded,
            ),
            PythonToolsFixture::tool(
                "greeter",
                "/artifacts/textkit",
                "Greeter",
                ResolvedContextToolPythonInterpreter::Embedded,
            ),
        ]);
        let (_, registrations) = PythonToolsFixture::generated(&ctx);

        assert_eq!(
            registrations
                .matches("Executor :: < agentc_executor_python")
                .count(),
            2
        );
        assert!(registrations.contains("/artifacts/mathkit"));
        assert!(registrations.contains("/artifacts/textkit"));
        assert!(registrations.contains("/artifacts/mathkit/.venv/site-packages"));
        assert!(registrations.contains("/artifacts/textkit/.venv/site-packages"));
    }

    #[test]
    fn executor_uses_four_workers_queue_and_cancellation() {
        let ctx = PythonToolsFixture::context([PythonToolsFixture::tool(
            "adder",
            "/artifacts/adder",
            "Adder",
            ResolvedContextToolPythonInterpreter::Embedded,
        )]);
        let (_, registrations) = PythonToolsFixture::generated(&ctx);

        assert!(registrations.contains(". workers (4)"));
        assert!(registrations.contains(". with_tools ()"));
        assert!(registrations.contains(". queue_capacity (32)"));
        assert!(registrations.contains(". cancellation (shutdown . clone ())"));
        assert!(!registrations.contains("standard_environment"));
        assert!(!registrations.contains("with_http"));
        assert!(!registrations.contains("with_fs"));
    }

    #[test]
    fn registration_passes_the_export_name() {
        let ctx = PythonToolsFixture::context([PythonToolsFixture::tool(
            "weather",
            "/artifacts/weather",
            "Weather",
            ResolvedContextToolPythonInterpreter::Embedded,
        )]);
        let (_, registrations) = PythonToolsFixture::generated(&ctx);

        assert!(registrations.contains(". export_name (\"Weather\")"));
        assert!(!registrations.contains("tool_name"));
    }

    #[test]
    fn absent_python_tools_generate_no_imports_or_registrations() {
        let ctx = PythonToolsFixture::context([]);
        let (imports, registrations) = PythonToolsFixture::generated(&ctx);

        assert!(imports.is_empty());
        assert!(registrations.is_empty());
        assert!(
            ToolsCodeGen::features(&ctx)
                .to_string()
                .is_empty()
        );
    }
}
