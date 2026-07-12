// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter, Result as FmtResult};

use agentc_compiler::{
    compiler::cargo::CargoCompiler,
    generator::{
        blocks::{
            BlockSet,
            codegen::CodeGenBlock,
            template::{
                ExtensionPointSpec, FileSpec, Reducer, TemplateBlock, TemplateBlockManifest,
                TemplateFragmentBlock,
            },
        },
        extension::{Contribution, reducers},
    },
};

use crate::{
    archetype::{
        standalone::codegen::{
            build_script::BuildScriptCodeGen,
            cargo::{
                A2aClientCargoFragment, CargoDependenciesExtensionPoint,
                CargoDependencyContribution, CargoPatchContribution, CargoPatchesExtensionPoint,
            },
            cli::{CliModCodeGen, config::CliConfigCodeGen, shutdown::CliShutdownCodeGen},
            config::ConfigCodeGen,
            entrypoint::EntrypointCodeGen,
            migrator::MigratorCodeGen,
        },
        traits::Archetype,
        types::ResolvedArchetype,
    },
    composition::GenerationContribution,
    context::ResolvedContext,
    errors::BlocksError,
    feature::{ArchetypeStandalone, Cli, GenerationFeatureSet, HttpServer, LongLivedProcess},
    fields::FieldsSpec,
    runtime::EMBEDDED_RUNTIME,
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Os {
    Linux,
    MacOS,
    Windows,
}

impl Os {
    pub fn current() -> Result<Self, BlocksError> {
        match std::env::consts::OS {
            "linux" => Ok(Self::Linux),
            "macos" => Ok(Self::MacOS),
            "windows" => Ok(Self::Windows),
            other => Err(BlocksError::invalid(format!("unsupported OS: {}", other))),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Arch {
    X86_64,
    Aarch64,
}

impl Arch {
    pub fn current() -> Result<Self, BlocksError> {
        match std::env::consts::ARCH {
            "x86_64" => Ok(Self::X86_64),
            "aarch64" => Ok(Self::Aarch64),
            other => Err(BlocksError::invalid(format!("unsupported architecture: {}", other))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetTriple(pub &'static str);

impl TargetTriple {
    pub fn as_str(&self) -> &'static str {
        self.0
    }
}

impl Display for TargetTriple {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.0)
    }
}

impl From<(Os, Arch)> for TargetTriple {
    fn from((os, arch): (Os, Arch)) -> Self {
        match (os, arch) {
            (Os::Linux, Arch::X86_64) => Self("x86_64-unknown-linux-gnu"),
            (Os::Linux, Arch::Aarch64) => Self("aarch64-unknown-linux-gnu"),
            (Os::MacOS, Arch::X86_64) => Self("x86_64-apple-darwin"),
            (Os::MacOS, Arch::Aarch64) => Self("aarch64-apple-darwin"),
            (Os::Windows, Arch::X86_64) => Self("x86_64-pc-windows-msvc"),
            (Os::Windows, Arch::Aarch64) => Self("aarch64-pc-windows-msvc"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StandaloneArchetypeConfig {
    pub os: Option<Os>,
    pub arch: Option<Arch>,
}

impl StandaloneArchetypeConfig {
    pub fn target_triple(&self) -> Result<Option<TargetTriple>, BlocksError> {
        match (self.os, self.arch) {
            (Some(os), Some(arch)) => Ok(Some(TargetTriple::from((os, arch)))),
            (None, None) => Ok(None),
            _ => Err(BlocksError::invalid("both os and arch must be specified together")),
        }
    }
}

pub struct StandaloneArchetype;

impl Archetype for StandaloneArchetype {
    type Config = StandaloneArchetypeConfig;

    fn name(&self) -> &str {
        "standalone"
    }

    fn resolve(
        &self,
        context: ResolvedContext,
        config: Self::Config,
    ) -> Result<ResolvedArchetype, BlocksError> {
        let fields = FieldsSpec::collect_from(&context);

        Ok(ResolvedArchetype {
            name: self.name().to_string(),
            compiler: Box::new(CargoCompiler::new()),
            target: config
                .target_triple()?
                .map(|t| t.to_string()),
            contribution: GenerationContribution::new()
                .with_blocks(
                    BlockSet::new()
                        .add(
                            TemplateBlock::builder()
                                .with_manifest(TemplateBlockManifest {
                                    id: "cargo_toml".to_string(),
                                    files: vec![FileSpec {
                                        path: "Cargo.toml".to_string(),
                                        template: "cargo_toml".to_string(),
                                        condition: None,
                                    }],
                                    extension_points: vec![ExtensionPointSpec {
                                        name: "tools::features".to_string(),
                                        reducer: Reducer::JoinComma,
                                    }],
                                    slot_fills: vec![],
                                    description: None,
                                })
                                .typed_extension_point(CargoDependenciesExtensionPoint::new(
                                    "cargo::dependencies",
                                    env!("CARGO_PKG_VERSION"),
                                ))
                                .typed_extension_point(CargoPatchesExtensionPoint::new(
                                    "cargo::patches",
                                ))
                                .with_template(
                                    "cargo_toml",
                                    include_str!("templates/Cargo.toml.j2"),
                                )
                                .with_var("runtime_version", env!("CARGO_PKG_VERSION"))
                                .build(),
                        )
                        .add(
                            TemplateBlock::builder()
                                .with_manifest(TemplateBlockManifest {
                                    id: "rust-toolchain_toml".to_string(),
                                    files: vec![FileSpec {
                                        path: "rust-toolchain.toml".to_string(),
                                        template: "rust-toolchain_toml".to_string(),
                                        condition: None,
                                    }],
                                    extension_points: vec![],
                                    slot_fills: vec![],
                                    description: None,
                                })
                                .with_template(
                                    "rust-toolchain_toml",
                                    include_str!("templates/rust-toolchain.toml.j2"),
                                )
                                .build(),
                        )
                        .add(
                            CodeGenBlock::builder()
                                .id("build_rs")
                                .build(BuildScriptCodeGen),
                        )
                        .add(
                            TemplateFragmentBlock::builder()
                                .id("a2a_client_cargo")
                                .contribute(Contribution::<CargoDependencyContribution>::strict(
                                    "cargo::dependencies",
                                ))
                                .contribute(Contribution::<CargoPatchContribution>::strict(
                                    "cargo::patches",
                                ))
                                .build(A2aClientCargoFragment),
                        )
                        .add(
                            CodeGenBlock::builder()
                                .id("migrator_rs")
                                .extension_point("migrator::use", reducers::concat)
                                .extension_point("migrator::migrations", reducers::concat)
                                .build(MigratorCodeGen),
                        )
                        .add(
                            CodeGenBlock::builder()
                                .id("config_rs")
                                .extension_point("config::use", reducers::concat)
                                .extension_point("config::fields", reducers::concat)
                                .extension_point("config::impls", reducers::concat)
                                .extension_point("config::loader", reducers::concat)
                                .extension_point("config::mapper", reducers::concat)
                                .build(ConfigCodeGen { fields: fields.clone() }),
                        )
                        .add(
                            CodeGenBlock::builder()
                                .id("cli_mod")
                                .extension_point("cli::mod::use", reducers::concat)
                                .extension_point("cli::mod::variants", reducers::concat)
                                .extension_point("cli::mod::arms", reducers::concat)
                                .build(CliModCodeGen),
                        )
                        .add(
                            CodeGenBlock::builder()
                                .id("cli_shutdown")
                                .build(CliShutdownCodeGen),
                        )
                        .add(
                            CodeGenBlock::builder()
                                .id("cli_config")
                                .build(CliConfigCodeGen),
                        )
                        .add(
                            CodeGenBlock::builder()
                                .id("main_rs")
                                .extension_point("main::modules", reducers::concat)
                                .build(EntrypointCodeGen),
                        )
                        .into_inner(),
                )
                .with_embedded_assets(EMBEDDED_RUNTIME.iter().collect())
                .with_provides(
                    GenerationFeatureSet::new()
                        .with::<ArchetypeStandalone>()
                        .with::<Cli>()
                        .with::<LongLivedProcess>()
                        .with_if::<HttpServer>(context.http_server.is_some()),
                ),
        })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::contributions::dependency::RuntimeDependencyContribution;
    use agentc_compiler::generator::{
        context::GenerationContext,
        extension::{ErasedContributionValue, ExtensionRegistry},
        vfs::VirtualFileSystem,
    };
    use std::collections::HashMap;

    fn context(http_server: Option<serde_json::Value>) -> ResolvedContext {
        serde_json::from_value(json!({
            "slug": "assistant",
            "agent_name": "assistant",
            "runtime": { "default_tenant_id": "default" },
            "providers": [],
            "agent": {
                "version": "0.1.0",
                "description": null,
                "prompt": null,
                "capabilities": null,
                "capability_policy": null,
                "model": { "provider": "anthropic", "name": "claude" }
            },
            "blocks": {},
            "tools": {},
            "skills": {},
            "http_server": http_server
        }))
        .unwrap()
    }

    #[test]
    fn provides_cli_and_long_lived_process_always() {
        let resolved = StandaloneArchetype
            .resolve(context(None), StandaloneArchetypeConfig::default())
            .unwrap();

        assert!(
            resolved
                .contribution
                .provides
                .contains::<ArchetypeStandalone>()
        );
        assert!(
            resolved
                .contribution
                .provides
                .contains::<Cli>()
        );
        assert!(
            resolved
                .contribution
                .provides
                .contains::<LongLivedProcess>()
        );
        assert!(
            !resolved
                .contribution
                .provides
                .contains::<HttpServer>()
        );
    }

    #[test]
    fn provides_http_server_only_when_configured() {
        let resolved = StandaloneArchetype
            .resolve(
                context(Some(json!({ "host": "0.0.0.0", "port": 8080, "max_request_size": 2097152, "protocols": [] }))),
                StandaloneArchetypeConfig::default(),
            )
            .unwrap();

        assert!(
            resolved
                .contribution
                .provides
                .contains::<HttpServer>()
        );
    }

    #[tokio::test]
    async fn generated_cargo_toml_has_no_react_or_ag_ui_references() {
        let resolved = StandaloneArchetype
            .resolve(
                context(Some(json!({ "host": "0.0.0.0", "port": 8080, "max_request_size": 2097152, "protocols": [] }))),
                StandaloneArchetypeConfig::default(),
            )
            .unwrap();

        let cargo_toml_block = resolved
            .contribution
            .blocks
            .iter()
            .find(|block| block.id() == "cargo_toml")
            .expect("cargo_toml block is registered");

        let ctx = GenerationContext::new(context(Some(
            json!({ "host": "0.0.0.0", "port": 8080, "max_request_size": 2097152, "protocols": [] }),
        )));
        let mut vfs = VirtualFileSystem::new();

        cargo_toml_block
            .render(&ctx, &ExtensionRegistry::empty(), &mut vfs)
            .await
            .unwrap();

        let content = vfs
            .get("Cargo.toml")
            .expect("Cargo.toml is generated");

        assert!(!content.contains("agentc-agent-react"));
        assert!(!content.contains("agentc-protocol-ag-ui"));
        assert!(!content.contains("has_ag_ui_protocol"));
    }

    #[tokio::test]
    async fn contributes_a2a_client_dependency_without_declared_a2a_tool() {
        let resolved = StandaloneArchetype
            .resolve(context(None), StandaloneArchetypeConfig::default())
            .unwrap();

        let dependency = resolved
            .contribution
            .blocks
            .iter()
            .find(|block| block.id() == "a2a_client_cargo")
            .expect("a2a client cargo block is registered")
            .render_contribution(&GenerationContext::new(context(None)), "cargo::dependencies")
            .await
            .unwrap()
            .downcast::<CargoDependencyContribution>()
            .unwrap();

        assert!(matches!(
            dependency,
            CargoDependencyContribution::Runtime(dependency)
                if dependency.name == "agentc-protocol-a2a"
                    && dependency.default_features == Some(false)
                    && dependency.features.len() == 1
                    && dependency.features.contains("client")
        ));
    }

    #[tokio::test]
    async fn generated_cargo_toml_includes_a2a_client_dependency() {
        let resolved = StandaloneArchetype
            .resolve(context(None), StandaloneArchetypeConfig::default())
            .unwrap();
        let ctx = GenerationContext::new(context(None));
        let cargo_toml_block = resolved
            .contribution
            .blocks
            .iter()
            .find(|block| block.id() == "cargo_toml")
            .expect("cargo_toml block is registered");

        let registry = ExtensionRegistry::resolve(
            vec![
                Box::new(CargoDependenciesExtensionPoint::new(
                    "cargo::dependencies",
                    env!("CARGO_PKG_VERSION"),
                )),
                Box::new(CargoPatchesExtensionPoint::new("cargo::patches")),
            ],
            HashMap::from([
                (
                    "cargo::dependencies".to_string(),
                    vec![ErasedContributionValue::new(
                        CargoDependencyContribution::runtime(
                            RuntimeDependencyContribution::new("agentc-protocol-a2a")
                                .default_features(false)
                                .feature("client"),
                        ),
                    )],
                ),
                (
                    "cargo::patches".to_string(),
                    vec![ErasedContributionValue::new(
                        CargoPatchContribution::runtime(RuntimeDependencyContribution::new(
                            "agentc-protocol-a2a",
                        )),
                    )],
                ),
            ]),
        )
        .unwrap();
        let mut vfs = VirtualFileSystem::new();

        cargo_toml_block
            .render(&ctx, &registry, &mut vfs)
            .await
            .unwrap();

        let content = vfs
            .get("Cargo.toml")
            .expect("Cargo.toml is generated");

        assert!(content.contains(&format!(
            "agentc-protocol-a2a = {{ version = \"{}\", default-features = false, features = [\"client\"] }}",
            env!("CARGO_PKG_VERSION"),
        )));
    }
}
