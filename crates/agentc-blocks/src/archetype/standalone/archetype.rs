// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter, Result as FmtResult};

use agentc_compiler::{
    compiler::cargo::CargoCompiler,
    generator::{
        blocks::{
            codegen::CodeGenBlock,
            template::{
                ExtensionPointSpec, FileSpec, Reducer, TemplateBlock, TemplateBlockManifest,
            },
            traits::Block,
        },
        extension::{Contribution, reducers},
    },
};

use crate::{
    archetype::{
        standalone::{
            codegen::{
                agent_rs::AgentRsCodeGen, build_rs::BuildRsCodeGen, cli_config::CliConfigCodeGen,
                cli_mod::CliModCodeGen, cli_shutdown::CliShutdownCodeGen, cli_run::CliRunCodeGen,
                cli_serve::CliServeCodeGen, config_rs::ConfigRsCodeGen, main_rs::MainRsCodeGen,
                migrator_rs::MigratorRsCodeGen, protocol_ag_ui::ProtocolAgUiCodeGen, server_rs::ServerRsCodeGen,
            },
            fields::{FieldSpec, FieldsSpec},
        },
        traits::Archetype,
        types::ResolvedArchetype,
    },
    context::{
        ResolvedContext, ResolvedContextHttpServerProtocolAgUi, ResolvedContextProvider,
        ResolvedContextToolKind,
    },
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

impl StandaloneArchetype {
    fn cargo_toml() -> TemplateBlock {
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
            .build()
    }

    fn build_rs() -> CodeGenBlock<ResolvedContext> {
        CodeGenBlock::builder()
            .id("build_rs")
            .build(BuildRsCodeGen)
    }

    fn main_rs() -> CodeGenBlock<ResolvedContext> {
        CodeGenBlock::builder()
            .id("main_rs")
            .extension_point("main::modules", reducers::concat)
            .build(MainRsCodeGen)
    }

    fn cli_mod() -> CodeGenBlock<ResolvedContext> {
        CodeGenBlock::builder()
            .id("cli_mod")
            .extension_point("cli::mod::use", reducers::concat)
            .extension_point("cli::mod::variants", reducers::concat)
            .extension_point("cli::mod::arms", reducers::concat)
            .build(CliModCodeGen)
    }

    fn cli_shutdown() -> CodeGenBlock<ResolvedContext> {
        CodeGenBlock::builder()
            .id("cli_shutdown")
            .build(CliShutdownCodeGen)
    }

    fn cli_run() -> CodeGenBlock<ResolvedContext> {
        CodeGenBlock::builder()
            .id("cli_run")
            .build(CliRunCodeGen)
    }

    fn cli_config() -> CodeGenBlock<ResolvedContext> {
        CodeGenBlock::builder()
            .id("cli_config")
            .build(CliConfigCodeGen)
    }

    fn cli_serve() -> CodeGenBlock<ResolvedContext> {
        CodeGenBlock::builder()
            .id("cli_serve")
            .contribute(Contribution::strict("cli::mod::use"))
            .contribute(Contribution::strict("cli::mod::variants"))
            .contribute(Contribution::strict("cli::mod::arms"))
            .build(CliServeCodeGen)
    }

    fn agent_rs(fields: &FieldsSpec) -> CodeGenBlock<ResolvedContext> {
        CodeGenBlock::builder()
            .id("agent_rs")
            .extension_point("agent::use", reducers::concat)
            .extension_point("agent::tools", reducers::concat)
            .contribute(Contribution::strict("config::loader"))
            .contribute(Contribution::strict("config::mapper"))
            .contribute(Contribution::lenient("tools::features"))
            .build(AgentRsCodeGen { fields: fields.clone() })
    }

    fn migrator_rs() -> CodeGenBlock<ResolvedContext> {
        CodeGenBlock::builder()
            .id("migrator_rs")
            .extension_point("migrator::use", reducers::concat)
            .extension_point("migrator::migrations", reducers::concat)
            .build(MigratorRsCodeGen)
    }

    fn config_rs(fields: &FieldsSpec) -> CodeGenBlock<ResolvedContext> {
        CodeGenBlock::builder()
            .id("config_rs")
            .extension_point("config::use", reducers::concat)
            .extension_point("config::fields", reducers::concat)
            .extension_point("config::impls", reducers::concat)
            .extension_point("config::loader", reducers::concat)
            .extension_point("config::mapper", reducers::concat)
            .build(ConfigRsCodeGen { fields: fields.clone() })
    }

    fn server_rs(fields: &FieldsSpec) -> CodeGenBlock<ResolvedContext> {
        CodeGenBlock::builder()
            .id("server_rs")
            .extension_point("server::use", reducers::concat)
            .extension_point("server::routers", reducers::concat)
            .contribute(Contribution::strict("main::modules"))
            .build(ServerRsCodeGen { fields: fields.clone() })
    }

    fn protocol_ag_ui(
        config: &ResolvedContextHttpServerProtocolAgUi,
    ) -> CodeGenBlock<ResolvedContext> {
        CodeGenBlock::builder()
            .id("protocol_ag_ui")
            .contribute(Contribution::strict("server::routers"))
            .contribute(Contribution::strict("agent::features"))
            .build(ProtocolAgUiCodeGen { config: config.clone() })
    }

    fn build_field_spec(ctx: &ResolvedContext) -> FieldsSpec {
        let mut fields = Vec::new();

        fields.push(FieldSpec::new(&["default_tenant_id"], &ctx.runtime.default_tenant_id));

        for provider in &ctx.providers {
            match provider {
                ResolvedContextProvider::Anthropic(anthropic) => {
                    if let Some(config) = &anthropic.config {
                        if let Some(api_key) = &config.api_key {
                            fields.push(FieldSpec::new(
                                &["provider", "anthropic", "api_key"],
                                api_key,
                            ));
                        }
                        if let Some(base_url) = &config.base_url {
                            fields.push(FieldSpec::new(
                                &["provider", "anthropic", "base_url"],
                                base_url,
                            ));
                        }
                    }

                    if let Some(params) = &anthropic.params {
                        if let Some(v) = &params.max_tokens {
                            fields.push(FieldSpec::new(
                                &["provider", "anthropic", "params", "max_tokens"],
                                v,
                            ));
                        }
                        if let Some(v) = &params.temperature {
                            fields.push(FieldSpec::new(
                                &["provider", "anthropic", "params", "temperature"],
                                v,
                            ));
                        }
                        if let Some(v) = &params.top_p {
                            fields.push(FieldSpec::new(
                                &["provider", "anthropic", "params", "top_p"],
                                v,
                            ));
                        }
                        if let Some(v) = &params.top_k {
                            fields.push(FieldSpec::new(
                                &["provider", "anthropic", "params", "top_k"],
                                v,
                            ));
                        }
                        if let Some(v) = &params.stop_sequences {
                            fields.push(FieldSpec::new(
                                &["provider", "anthropic", "params", "stop_sequences"],
                                v,
                            ));
                        }
                        if let Some(v) = &params.frequency_penalty {
                            fields.push(FieldSpec::new(
                                &["provider", "anthropic", "params", "frequency_penalty"],
                                v,
                            ));
                        }
                        if let Some(v) = &params.presence_penalty {
                            fields.push(FieldSpec::new(
                                &["provider", "anthropic", "params", "presence_penalty"],
                                v,
                            ));
                        }
                        if let Some(v) = &params.seed {
                            fields.push(FieldSpec::new(
                                &["provider", "anthropic", "params", "seed"],
                                v,
                            ));
                        }
                        if let Some(v) = &params.provider_params {
                            fields.push(FieldSpec::new(
                                &["provider", "anthropic", "params", "provider_params"],
                                v,
                            ));
                        }
                    }

                    if let Some(models) = &anthropic.models {
                        for model in models {
                            if let Some(params) = &model.params {
                                let slug: String = model
                                    .name
                                    .chars()
                                    .map(|c| if c.is_alphanumeric() { c } else { '_' })
                                    .collect();
                                let slug = slug.as_str();
                                if let Some(v) = &params.max_tokens {
                                    fields.push(FieldSpec::new(
                                        &["provider", "anthropic", slug, "max_tokens"],
                                        v,
                                    ));
                                }
                                if let Some(v) = &params.temperature {
                                    fields.push(FieldSpec::new(
                                        &["provider", "anthropic", slug, "temperature"],
                                        v,
                                    ));
                                }
                                if let Some(v) = &params.top_p {
                                    fields.push(FieldSpec::new(
                                        &["provider", "anthropic", slug, "top_p"],
                                        v,
                                    ));
                                }
                                if let Some(v) = &params.top_k {
                                    fields.push(FieldSpec::new(
                                        &["provider", "anthropic", slug, "top_k"],
                                        v,
                                    ));
                                }
                                if let Some(v) = &params.stop_sequences {
                                    fields.push(FieldSpec::new(
                                        &["provider", "anthropic", slug, "stop_sequences"],
                                        v,
                                    ));
                                }
                                if let Some(v) = &params.frequency_penalty {
                                    fields.push(FieldSpec::new(
                                        &["provider", "anthropic", slug, "frequency_penalty"],
                                        v,
                                    ));
                                }
                                if let Some(v) = &params.presence_penalty {
                                    fields.push(FieldSpec::new(
                                        &["provider", "anthropic", slug, "presence_penalty"],
                                        v,
                                    ));
                                }
                                if let Some(v) = &params.seed {
                                    fields.push(FieldSpec::new(
                                        &["provider", "anthropic", slug, "seed"],
                                        v,
                                    ));
                                }
                                if let Some(v) = &params.provider_params {
                                    fields.push(FieldSpec::new(
                                        &["provider", "anthropic", slug, "provider_params"],
                                        v,
                                    ));
                                }
                            }
                        }
                    }
                }
                ResolvedContextProvider::OpenAi(openai) => {
                    if let Some(config) = &openai.config {
                        if let Some(api_key) = &config.api_key {
                            fields
                                .push(FieldSpec::new(&["provider", "openai", "api_key"], api_key));
                        }
                        if let Some(base_url) = &config.base_url {
                            fields.push(FieldSpec::new(
                                &["provider", "openai", "base_url"],
                                base_url,
                            ));
                        }
                    }

                    if let Some(params) = &openai.params {
                        if let Some(v) = &params.max_tokens {
                            fields.push(FieldSpec::new(
                                &["provider", "openai", "params", "max_tokens"],
                                v,
                            ));
                        }
                        if let Some(v) = &params.temperature {
                            fields.push(FieldSpec::new(
                                &["provider", "openai", "params", "temperature"],
                                v,
                            ));
                        }
                        if let Some(v) = &params.top_p {
                            fields.push(FieldSpec::new(
                                &["provider", "openai", "params", "top_p"],
                                v,
                            ));
                        }
                        if let Some(v) = &params.top_k {
                            fields.push(FieldSpec::new(
                                &["provider", "openai", "params", "top_k"],
                                v,
                            ));
                        }
                        if let Some(v) = &params.stop_sequences {
                            fields.push(FieldSpec::new(
                                &["provider", "openai", "params", "stop_sequences"],
                                v,
                            ));
                        }
                        if let Some(v) = &params.frequency_penalty {
                            fields.push(FieldSpec::new(
                                &["provider", "openai", "params", "frequency_penalty"],
                                v,
                            ));
                        }
                        if let Some(v) = &params.presence_penalty {
                            fields.push(FieldSpec::new(
                                &["provider", "openai", "params", "presence_penalty"],
                                v,
                            ));
                        }
                        if let Some(v) = &params.seed {
                            fields
                                .push(FieldSpec::new(&["provider", "openai", "params", "seed"], v));
                        }
                        if let Some(v) = &params.provider_params {
                            fields.push(FieldSpec::new(
                                &["provider", "openai", "params", "provider_params"],
                                v,
                            ));
                        }
                    }

                    if let Some(models) = &openai.models {
                        for model in models {
                            if let Some(params) = &model.params {
                                let slug: String = model
                                    .name
                                    .chars()
                                    .map(|c| if c.is_alphanumeric() { c } else { '_' })
                                    .collect();
                                let slug = slug.as_str();
                                if let Some(v) = &params.max_tokens {
                                    fields.push(FieldSpec::new(
                                        &["provider", "openai", slug, "max_tokens"],
                                        v,
                                    ));
                                }
                                if let Some(v) = &params.temperature {
                                    fields.push(FieldSpec::new(
                                        &["provider", "openai", slug, "temperature"],
                                        v,
                                    ));
                                }
                                if let Some(v) = &params.top_p {
                                    fields.push(FieldSpec::new(
                                        &["provider", "openai", slug, "top_p"],
                                        v,
                                    ));
                                }
                                if let Some(v) = &params.top_k {
                                    fields.push(FieldSpec::new(
                                        &["provider", "openai", slug, "top_k"],
                                        v,
                                    ));
                                }
                                if let Some(v) = &params.stop_sequences {
                                    fields.push(FieldSpec::new(
                                        &["provider", "openai", slug, "stop_sequences"],
                                        v,
                                    ));
                                }
                                if let Some(v) = &params.frequency_penalty {
                                    fields.push(FieldSpec::new(
                                        &["provider", "openai", slug, "frequency_penalty"],
                                        v,
                                    ));
                                }
                                if let Some(v) = &params.presence_penalty {
                                    fields.push(FieldSpec::new(
                                        &["provider", "openai", slug, "presence_penalty"],
                                        v,
                                    ));
                                }
                                if let Some(v) = &params.seed {
                                    fields.push(FieldSpec::new(
                                        &["provider", "openai", slug, "seed"],
                                        v,
                                    ));
                                }
                                if let Some(v) = &params.provider_params {
                                    fields.push(FieldSpec::new(
                                        &["provider", "openai", slug, "provider_params"],
                                        v,
                                    ));
                                }
                            }
                        }
                    }
                }
                ResolvedContextProvider::Ollama(ollama) => {
                    if let Some(config) = &ollama.config
                        && let Some(base_url) = &config.base_url {
                            fields.push(FieldSpec::new(
                                &["provider", "ollama", "base_url"],
                                base_url,
                            ));
                        }

                    if let Some(params) = &ollama.params {
                        if let Some(v) = &params.max_tokens {
                            fields.push(FieldSpec::new(
                                &["provider", "ollama", "params", "max_tokens"],
                                v,
                            ));
                        }
                        if let Some(v) = &params.temperature {
                            fields.push(FieldSpec::new(
                                &["provider", "ollama", "params", "temperature"],
                                v,
                            ));
                        }
                        if let Some(v) = &params.top_p {
                            fields.push(FieldSpec::new(
                                &["provider", "ollama", "params", "top_p"],
                                v,
                            ));
                        }
                        if let Some(v) = &params.top_k {
                            fields.push(FieldSpec::new(
                                &["provider", "ollama", "params", "top_k"],
                                v,
                            ));
                        }
                        if let Some(v) = &params.stop_sequences {
                            fields.push(FieldSpec::new(
                                &["provider", "ollama", "params", "stop_sequences"],
                                v,
                            ));
                        }
                        if let Some(v) = &params.frequency_penalty {
                            fields.push(FieldSpec::new(
                                &["provider", "ollama", "params", "frequency_penalty"],
                                v,
                            ));
                        }
                        if let Some(v) = &params.presence_penalty {
                            fields.push(FieldSpec::new(
                                &["provider", "ollama", "params", "presence_penalty"],
                                v,
                            ));
                        }
                        if let Some(v) = &params.seed {
                            fields
                                .push(FieldSpec::new(&["provider", "ollama", "params", "seed"], v));
                        }
                        if let Some(v) = &params.provider_params {
                            fields.push(FieldSpec::new(
                                &["provider", "ollama", "params", "provider_params"],
                                v,
                            ));
                        }
                    }

                    if let Some(models) = &ollama.models {
                        for model in models {
                            if let Some(params) = &model.params {
                                let slug: String = model
                                    .name
                                    .chars()
                                    .map(|c| if c.is_alphanumeric() { c } else { '_' })
                                    .collect();
                                let slug = slug.as_str();
                                if let Some(v) = &params.max_tokens {
                                    fields.push(FieldSpec::new(
                                        &["provider", "ollama", slug, "max_tokens"],
                                        v,
                                    ));
                                }
                                if let Some(v) = &params.temperature {
                                    fields.push(FieldSpec::new(
                                        &["provider", "ollama", slug, "temperature"],
                                        v,
                                    ));
                                }
                                if let Some(v) = &params.top_p {
                                    fields.push(FieldSpec::new(
                                        &["provider", "ollama", slug, "top_p"],
                                        v,
                                    ));
                                }
                                if let Some(v) = &params.top_k {
                                    fields.push(FieldSpec::new(
                                        &["provider", "ollama", slug, "top_k"],
                                        v,
                                    ));
                                }
                                if let Some(v) = &params.stop_sequences {
                                    fields.push(FieldSpec::new(
                                        &["provider", "ollama", slug, "stop_sequences"],
                                        v,
                                    ));
                                }
                                if let Some(v) = &params.frequency_penalty {
                                    fields.push(FieldSpec::new(
                                        &["provider", "ollama", slug, "frequency_penalty"],
                                        v,
                                    ));
                                }
                                if let Some(v) = &params.presence_penalty {
                                    fields.push(FieldSpec::new(
                                        &["provider", "ollama", slug, "presence_penalty"],
                                        v,
                                    ));
                                }
                                if let Some(v) = &params.seed {
                                    fields.push(FieldSpec::new(
                                        &["provider", "ollama", slug, "seed"],
                                        v,
                                    ));
                                }
                                if let Some(v) = &params.provider_params {
                                    fields.push(FieldSpec::new(
                                        &["provider", "ollama", slug, "provider_params"],
                                        v,
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }

        fields.push(FieldSpec::new(&["agent", "model", "provider"], &ctx.agent.model.provider));
        fields.push(FieldSpec::new(&["agent", "model", "name"], &ctx.agent.model.name));

        if let Some(capabilities) = &ctx.agent.capabilities {
            fields.push(FieldSpec::new(&["agent", "capabilities"], capabilities));
        }

        for (tool_name, tool) in &ctx.tools {
            match &tool.kind {
                ResolvedContextToolKind::Javascript(_) => {
                    fields.push(FieldSpec::new(&["tool", tool_name, "enabled"], &tool.enabled));

                    for (config_key, config_value) in &tool.config {
                        fields.push(FieldSpec::new(&["tool", tool_name, config_key], config_value));
                    }
                }

                // MCP tool loader calls are contributed to `config::loader` by AgentRsCodeGen.
                ResolvedContextToolKind::Mcp(_) => {}

                // Bash tools have no runtime-configurable fields beyond what is baked at compile time.
                ResolvedContextToolKind::Bash(_) => {}

                ResolvedContextToolKind::Python(_) => {
                    fields.push(FieldSpec::new(&["tool", tool_name, "enabled"], &tool.enabled));

                    for (config_key, config_value) in &tool.config {
                        fields.push(FieldSpec::new(&["tool", tool_name, config_key], config_value));
                    }
                }
            }
        }

        if let Some(http_server) = &ctx.http_server {
            fields.push(FieldSpec::new(&["server", "host"], &http_server.host));
            fields.push(FieldSpec::new(&["server", "port"], &http_server.port));
        }

        fields.into()
    }
}

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
        let fields = Self::build_field_spec(&context);
        let mut blocks: Vec<Box<dyn Block<ResolvedContext>>> = vec![
            Box::new(Self::cargo_toml()),
            Box::new(Self::build_rs()),
            Box::new(Self::migrator_rs()),
            Box::new(Self::config_rs(&fields)),
            Box::new(Self::agent_rs(&fields)),
            Box::new(Self::cli_mod()),
            Box::new(Self::cli_shutdown()),
            Box::new(Self::cli_run()),
            Box::new(Self::cli_config()),
            Box::new(Self::main_rs()),
        ];

        if let Some(http_server) = &context.http_server {
            blocks.push(Box::new(Self::server_rs(&fields)));
            blocks.push(Box::new(Self::cli_serve()));

            if let Some(ag_ui) = http_server
                .protocols
                .iter()
                .filter_map(|p| p.as_ag_ui())
                .next()
            {
                blocks.push(Box::new(Self::protocol_ag_ui(ag_ui)))
            }
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
