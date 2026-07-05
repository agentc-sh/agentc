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
            template::{ExtensionPointSpec, FileSpec, Reducer, TemplateBlock, TemplateBlockManifest},
        },
        extension::{Contribution, reducers},
    },
};

use crate::{
    archetype::{
        standalone::{
            codegen::{
                agent_rs::AgentRsCodeGen,
                build_rs::BuildRsCodeGen,
                cli_config::CliConfigCodeGen,
                cli_mod::CliModCodeGen,
                cli_run::CliRunCodeGen,
                cli_serve::CliServeCodeGen,
                cli_shutdown::CliShutdownCodeGen,
                config_rs::ConfigRsCodeGen,
                main_rs::MainRsCodeGen,
                migrator_rs::MigratorRsCodeGen,
                protocol_ag_ui::ProtocolAgUiCodeGen,
                server_rs::ServerRsCodeGen,
            },
            fields::FieldsSpec,
        },
        traits::Archetype,
        types::ResolvedArchetype,
    },
    context::ResolvedContext,
    errors::BlocksError,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
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

        let ag_ui = context
            .http_server
            .as_ref()
            .and_then(|server| {
                server.protocols
                    .iter()
                    .filter_map(|p| p.as_ag_ui())
                    .next()
                    .cloned()
            });

        let mut blocks = BlockSet::new()
            .add(
                TemplateBlock::builder()
                    .with_manifest(TemplateBlockManifest {
                        id: "cargo_toml".to_string(),
                        files: vec![FileSpec {
                            path: "Cargo.toml".to_string(),
                            template: "cargo_toml".to_string(),
                            condition: None,
                        }],
                        extension_points: vec![
                            ExtensionPointSpec {
                                name: "cargo::dependencies".to_string(),
                                reducer: Reducer::Concat,
                            },
                            ExtensionPointSpec {
                                name: "agent::features".to_string(),
                                reducer: Reducer::JoinComma,
                            },
                            ExtensionPointSpec {
                                name: "tools::features".to_string(),
                                reducer: Reducer::JoinComma,
                            },
                        ],
                        slot_fills: vec![],
                        description: None,
                    })
                    .with_template("cargo_toml", include_str!("templates/Cargo.toml.j2"))
                    .with_var("runtime_version", env!("CARGO_PKG_VERSION"))
                    .build(),
            )
            .add(CodeGenBlock::builder().id("build_rs").build(BuildRsCodeGen))
            .add(
                CodeGenBlock::builder()
                    .id("migrator_rs")
                    .extension_point("migrator::use", reducers::concat)
                    .extension_point("migrator::migrations", reducers::concat)
                    .build(MigratorRsCodeGen),
            )
            .add(
                CodeGenBlock::builder()
                    .id("config_rs")
                    .extension_point("config::use", reducers::concat)
                    .extension_point("config::fields", reducers::concat)
                    .extension_point("config::impls", reducers::concat)
                    .extension_point("config::loader", reducers::concat)
                    .extension_point("config::mapper", reducers::concat)
                    .build(ConfigRsCodeGen { fields: fields.clone() }),
            )
            .add(
                CodeGenBlock::builder()
                    .id("agent_rs")
                    .extension_point("agent::use", reducers::concat)
                    .extension_point("agent::tools", reducers::concat)
                    .contribute(Contribution::strict("config::loader"))
                    .contribute(Contribution::strict("config::mapper"))
                    .contribute(Contribution::lenient("tools::features"))
                    .build(AgentRsCodeGen { fields: fields.clone() }),
            )
            .add(
                CodeGenBlock::builder()
                    .id("cli_mod")
                    .extension_point("cli::mod::use", reducers::concat)
                    .extension_point("cli::mod::variants", reducers::concat)
                    .extension_point("cli::mod::arms", reducers::concat)
                    .build(CliModCodeGen),
            )
            .add(CodeGenBlock::builder().id("cli_shutdown").build(CliShutdownCodeGen))
            .add(CodeGenBlock::builder().id("cli_run").build(CliRunCodeGen))
            .add(CodeGenBlock::builder().id("cli_config").build(CliConfigCodeGen))
            .add(
                CodeGenBlock::builder()
                    .id("main_rs")
                    .extension_point("main::modules", reducers::concat)
                    .build(MainRsCodeGen),
            )
            .add_if(
                context.http_server.is_some(),
                CodeGenBlock::builder()
                    .id("server_rs")
                    .extension_point("server::use", reducers::concat)
                    .extension_point("server::routers", reducers::concat)
                    .contribute(Contribution::strict("main::modules"))
                    .build(ServerRsCodeGen { fields: fields.clone() }),
            )
            .add_if(
                context.http_server.is_some(),
                CodeGenBlock::builder()
                    .id("cli_serve")
                    .contribute(Contribution::strict("cli::mod::use"))
                    .contribute(Contribution::strict("cli::mod::variants"))
                    .contribute(Contribution::strict("cli::mod::arms"))
                    .build(CliServeCodeGen),
            )
            .into_inner();

        if let Some(ag_ui) = ag_ui {
            blocks.push(Box::new(
                CodeGenBlock::builder()
                    .id("protocol_ag_ui")
                    .contribute(Contribution::strict("server::routers"))
                    .contribute(Contribution::strict("agent::features"))
                    .build(ProtocolAgUiCodeGen { config: ag_ui }),
            ));
        }

        Ok(ResolvedArchetype {
            name: self.name().to_string(),
            compiler: Box::new(CargoCompiler::new()),
            blocks,
            target: config
                .target_triple()?
                .map(|t| t.to_string()),
            embedded_assets: EMBEDDED_RUNTIME,
        })
    }
}
