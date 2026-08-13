// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

pub mod agent;
pub mod cargo;
pub mod cli_migrate;
pub mod cli_run;
pub mod cli_serve;
pub mod migrator;
pub mod server;

use serde::{Deserialize, Serialize};

use agentc_compiler::generator::{
    blocks::{BlockSet, codegen::CodeGenBlock, fragment::FragmentBlock},
    extension::{Contribution, RenderedTokenStream, reducers},
};

use crate::{
    composition::GenerationContribution,
    config::{
        fields::FieldsSpec,
        sections::{
            database::DatabaseSection, pubsub::PubSubSection, task_queue::TaskQueueSection,
        },
    },
    context::ResolvedContext,
    contributions::dependency::{CargoDependencies, CargoPatches},
    errors::BlocksError,
    feature::{
        GenerationFeatureSet, GraphReAct, HttpServer, ProtocolA2a, ProtocolAgUi, Streaming,
        SupportsA2a, SupportsAgUi,
    },
    graph::{
        codegen::tools::javascript::{
            FilesystemTypescriptCargoFragment, HttpTypescriptCargoFragment,
            JavascriptToolCargoFragment,
        },
        react::{
            agent::AgentCodeGen,
            cargo::{ReActCargoFragment, ReActDatabaseCargoFragment, ReActFeatureCargoFragment},
            cli_migrate::CliMigrateCodeGen,
            cli_run::CliRunCodeGen,
            cli_serve::CliServeCodeGen,
            migrator::MigratorCodeGen,
            server::ServerCodeGen,
        },
        traits::AgentGraph,
        types::ResolvedGraph,
    },
    types::RuntimeValue,
};

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ReActGraphConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<ReActGraphModelConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ReActGraphModelConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout: Option<RuntimeValue<u64>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry: Option<ReActGraphModelRetryConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReActGraphModelRetryConfig {
    pub max_attempts: RuntimeValue<u32>,
    pub initial_backoff: RuntimeValue<u64>,
    pub max_backoff: RuntimeValue<u64>,
}

pub struct ReActGraph;

impl AgentGraph for ReActGraph {
    type Config = ReActGraphConfig;

    fn name(&self) -> &str {
        "react"
    }

    fn resolve(
        &self,
        context: ResolvedContext,
        config: Self::Config,
    ) -> Result<ResolvedGraph, BlocksError> {
        let fields = FieldsSpec::collect_from(&context);

        let mut core_blocks = BlockSet::new()
            .add(
                CodeGenBlock::builder()
                    .id("agent_rs")
                    .token_stream_extension_point("agent::use", reducers::concat)
                    .token_stream_extension_point("agent::tools", reducers::concat)
                    .contribute(Contribution::<RenderedTokenStream>::lenient("config::fields"))
                    .contribute(Contribution::<RenderedTokenStream>::lenient("config::impls"))
                    .contribute(Contribution::<RenderedTokenStream>::lenient("config::loader"))
                    .contribute(Contribution::<RenderedTokenStream>::lenient("config::mapper"))
                    .contribute(Contribution::<String>::lenient("tools::features"))
                    .build(AgentCodeGen { fields: fields.clone(), config }),
            )
            .add(
                CodeGenBlock::builder()
                    .id("cli_run")
                    .build(CliRunCodeGen),
            )
            .add(DatabaseSection::block("react_database_section"))
            .add(
                CodeGenBlock::builder()
                    .id("migrator_rs")
                    .contribute(Contribution::<RenderedTokenStream>::strict("main::modules"))
                    .build(MigratorCodeGen),
            )
            .add(
                CodeGenBlock::builder()
                    .id("cli_migrate")
                    .contribute(Contribution::<RenderedTokenStream>::strict("cli::mod::use"))
                    .contribute(Contribution::<RenderedTokenStream>::strict("cli::mod::variants"))
                    .contribute(Contribution::<RenderedTokenStream>::strict("cli::mod::arms"))
                    .build(CliMigrateCodeGen),
            )
            .add(
                FragmentBlock::builder()
                    .id("react_cargo")
                    .contribute(Contribution::<CargoDependencies>::strict("cargo::dependencies"))
                    .contribute(Contribution::<CargoPatches>::strict("cargo::patches"))
                    .build(ReActCargoFragment),
            )
            .add(
                FragmentBlock::builder()
                    .id("react_database_cargo")
                    .contribute(Contribution::<CargoDependencies>::strict("cargo::dependencies"))
                    .contribute(Contribution::<CargoPatches>::strict("cargo::patches"))
                    .build(ReActDatabaseCargoFragment),
            );

        if context.has_typescript_components() {
            core_blocks = core_blocks
                .add(
                    FragmentBlock::builder()
                        .id("javascript_tool_cargo")
                        .contribute(Contribution::<CargoDependencies>::strict(
                            "cargo::dependencies",
                        ))
                        .contribute(Contribution::<CargoPatches>::strict("cargo::patches"))
                        .build(JavascriptToolCargoFragment),
                )
                .add(
                    FragmentBlock::builder()
                        .id("http_typescript_cargo")
                        .contribute(Contribution::<CargoDependencies>::strict(
                            "cargo::dependencies",
                        ))
                        .build(HttpTypescriptCargoFragment),
                )
                .add(
                    FragmentBlock::builder()
                        .id("filesystem_typescript_cargo")
                        .contribute(Contribution::<CargoDependencies>::strict(
                            "cargo::dependencies",
                        ))
                        .build(FilesystemTypescriptCargoFragment),
                );
        }

        let server_integration = GenerationContribution::new()
            .with_blocks(
                BlockSet::new()
                    .add(
                        CodeGenBlock::builder()
                            .id("server_rs")
                            .token_stream_extension_point("server::use", reducers::concat)
                            .token_stream_extension_point("server::routers", reducers::concat)
                            .contribute(Contribution::<RenderedTokenStream>::strict("main::modules"))
                            .build(ServerCodeGen { fields: fields.clone() }),
                    )
                    .add(
                        CodeGenBlock::builder()
                            .id("cli_serve")
                            .contribute(Contribution::<RenderedTokenStream>::strict("cli::mod::use"))
                            .contribute(Contribution::<RenderedTokenStream>::strict("cli::mod::variants"))
                            .contribute(Contribution::<RenderedTokenStream>::strict("cli::mod::arms"))
                            .build(CliServeCodeGen),
                    )
                    .add(
                        FragmentBlock::builder()
                            .id("react_api_cargo")
                            .contribute(Contribution::<CargoDependencies>::strict(
                                "cargo::dependencies",
                            ))
                            .build(ReActFeatureCargoFragment::new("api")),
                    )
                    .add(TaskQueueSection::block("react_task_queue_section"))
                    .add(PubSubSection::block("react_pubsub_section"))
                    .into_inner(),
            )
            .with_requires(GenerationFeatureSet::new().with::<HttpServer>());

        let ag_ui_integration = GenerationContribution::new()
            .with_blocks(
                BlockSet::new()
                    .add(
                        FragmentBlock::builder()
                            .id("react_ag_ui_cargo")
                            .contribute(Contribution::<CargoDependencies>::strict(
                                "cargo::dependencies",
                            ))
                            .build(ReActFeatureCargoFragment::new("ag-ui")),
                    )
                    .into_inner(),
            )
            .with_requires(
                GenerationFeatureSet::new()
                    .with::<HttpServer>()
                    .with::<ProtocolAgUi>(),
            );

        let a2a_integration = GenerationContribution::new()
            .with_blocks(
                BlockSet::new()
                    .add(
                        FragmentBlock::builder()
                            .id("react_a2a_cargo")
                            .contribute(Contribution::<CargoDependencies>::strict(
                                "cargo::dependencies",
                            ))
                            .build(ReActFeatureCargoFragment::new("a2a")),
                    )
                    .into_inner(),
            )
            .with_requires(
                GenerationFeatureSet::new()
                    .with::<HttpServer>()
                    .with::<ProtocolA2a>(),
            );

        Ok(ResolvedGraph {
            name: self.name().to_string(),
            contribution: GenerationContribution::new()
                .with_blocks(core_blocks.into_inner())
                .with_provides(
                    GenerationFeatureSet::new()
                        .with::<GraphReAct>()
                        .with::<Streaming>()
                        .with::<SupportsAgUi>()
                        .with::<SupportsA2a>(),
                ),
            integrations: vec![server_integration, ag_ui_integration, a2a_integration],
        })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

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
    fn provides_graph_react_and_streaming() {
        let resolved = ReActGraph
            .resolve(context(None), ReActGraphConfig::default())
            .unwrap();

        assert!(
            resolved
                .contribution
                .provides
                .contains::<GraphReAct>()
        );
        assert!(
            resolved
                .contribution
                .provides
                .contains::<Streaming>()
        );
        assert!(
            resolved
                .contribution
                .provides
                .contains::<SupportsAgUi>()
        );
        assert!(
            resolved
                .contribution
                .provides
                .contains::<SupportsA2a>()
        );
    }

    #[test]
    fn core_contribution_has_no_http_dependent_blocks() {
        let resolved = ReActGraph
            .resolve(context(None), ReActGraphConfig::default())
            .unwrap();

        let ids = resolved
            .contribution
            .blocks
            .iter()
            .map(|block| block.id().to_string())
            .collect::<Vec<_>>();

        assert_eq!(
            ids,
            vec![
                "agent_rs",
                "cli_run",
                "react_database_section",
                "migrator_rs",
                "cli_migrate",
                "react_cargo",
                "react_database_cargo"
            ]
        );
        assert_eq!(resolved.integrations.len(), 3);
        assert!(
            resolved.integrations[0]
                .requires
                .contains::<HttpServer>()
        );
        assert!(
            resolved.integrations[1]
                .requires
                .contains::<ProtocolAgUi>()
        );
        assert!(
            resolved.integrations[2]
                .requires
                .contains::<ProtocolA2a>()
        );
    }

    #[test]
    fn the_server_integration_carries_the_api_feature_and_the_task_queue() {
        let resolved = ReActGraph
            .resolve(context(None), ReActGraphConfig::default())
            .unwrap();

        let ids = resolved.integrations[0]
            .blocks
            .iter()
            .map(|block| block.id().to_string())
            .collect::<Vec<_>>();

        assert_eq!(
            ids,
            vec![
                "server_rs",
                "cli_serve",
                "react_api_cargo",
                "react_task_queue_section",
                "react_pubsub_section"
            ]
        );
    }

    #[test]
    fn registers_the_typescript_fragments_only_with_a_typescript_component() {
        let without = ReActGraph
            .resolve(context(None), ReActGraphConfig::default())
            .unwrap();

        assert!(
            !without
                .contribution
                .blocks
                .iter()
                .any(|block| block.id() == "http_typescript_cargo")
        );
        assert!(
            !without
                .contribution
                .blocks
                .iter()
                .any(|block| block.id() == "filesystem_typescript_cargo")
        );

        let mut ctx = context(None);

        ctx.tools.insert(
            "search".to_string(),
            serde_json::from_value(json!({
                "name": "search",
                "description": null,
                "enabled": true,
                "capabilities": [],
                "config": {},
                "kind": {
                    "kind": "javascript",
                    "bundle_path": "/artifacts/search/dist/index.js",
                    "export_name": "search"
                }
            }))
            .unwrap(),
        );

        let with = ReActGraph
            .resolve(ctx, ReActGraphConfig::default())
            .unwrap();

        assert!(
            with.contribution
                .blocks
                .iter()
                .any(|block| block.id() == "javascript_tool_cargo")
        );
        assert!(
            with.contribution
                .blocks
                .iter()
                .any(|block| block.id() == "http_typescript_cargo")
        );
        assert!(
            with.contribution
                .blocks
                .iter()
                .any(|block| block.id() == "filesystem_typescript_cargo")
        );
    }
}
