// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

pub mod agent;
pub mod cargo;
pub mod cli_run;
pub mod cli_serve;
pub mod migrations;
pub mod server;

use serde::{Deserialize, Serialize};

use agentc_compiler::generator::{
    blocks::{
        BlockSet,
        codegen::CodeGenBlock,
        template::TemplateFragmentBlock,
    },
    extension::{Contribution, reducers},
};

use crate::{
    composition::{GenerationContribution, OptionalGenerationContribution},
    context::ResolvedContext,
    errors::BlocksError,
    feature::{GenerationFeatureSet, GraphReAct, HttpServer, Streaming},
    fields::FieldsSpec,
    graph::{
        react::{
            agent::AgentCodeGen, cargo::ReActCargoFragment, cli_run::CliRunCodeGen,
            cli_serve::CliServeCodeGen, migrations::ReActMigrationsCodeGen, server::ServerCodeGen,
        },
        traits::AgentGraph,
        types::ResolvedGraph,
    },
};

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ReActGraphConfig {}

pub struct ReActGraph;

impl AgentGraph for ReActGraph {
    type Config = ReActGraphConfig;

    fn name(&self) -> &str {
        "react"
    }

    fn resolve(
        &self,
        context: ResolvedContext,
        _config: Self::Config,
    ) -> Result<ResolvedGraph, BlocksError> {
        let fields = FieldsSpec::collect_from(&context);

        let has_ag_ui = context
            .http_server
            .as_ref()
            .is_some_and(|server| server.protocols.iter().any(|p| p.as_ag_ui().is_some()));

        let core_blocks = BlockSet::new()
            .add(
                CodeGenBlock::builder()
                    .id("agent_rs")
                    .extension_point("agent::use", reducers::concat)
                    .extension_point("agent::tools", reducers::concat)
                    .contribute(Contribution::strict("config::loader"))
                    .contribute(Contribution::strict("config::mapper"))
                    .contribute(Contribution::lenient("tools::features"))
                    .build(AgentCodeGen { fields: fields.clone() }),
            )
            .add(
                CodeGenBlock::builder()
                    .id("cli_run")
                    .build(CliRunCodeGen),
            )
            .add(
                CodeGenBlock::builder()
                    .id("react_migrations")
                    .contribute(Contribution::strict("migrator::use"))
                    .contribute(Contribution::strict("migrator::migrations"))
                    .build(ReActMigrationsCodeGen),
            )
            .add(
                TemplateFragmentBlock::builder()
                    .id("react_cargo")
                    .contribute(Contribution::strict("cargo::dependencies"))
                    .contribute(Contribution::strict("cargo::patches"))
                    .build(ReActCargoFragment { has_ag_ui }),
            )
            .into_inner();

        let server_integration = OptionalGenerationContribution::new(
            GenerationContribution::new()
                .with_blocks(
                    BlockSet::new()
                        .add(
                            CodeGenBlock::builder()
                                .id("server_rs")
                                .extension_point("server::use", reducers::concat)
                                .extension_point("server::routers", reducers::concat)
                                .contribute(Contribution::strict("main::modules"))
                                .build(ServerCodeGen { fields: fields.clone() }),
                        )
                        .add(
                            CodeGenBlock::builder()
                                .id("cli_serve")
                                .contribute(Contribution::strict("cli::mod::use"))
                                .contribute(Contribution::strict("cli::mod::variants"))
                                .contribute(Contribution::strict("cli::mod::arms"))
                                .build(CliServeCodeGen),
                        )
                        .into_inner(),
                )
                .with_requires(GenerationFeatureSet::new().with::<HttpServer>()),
        );

        Ok(ResolvedGraph {
            name: self.name().to_string(),
            contribution: GenerationContribution::new()
                .with_blocks(core_blocks)
                .with_provides(
                    GenerationFeatureSet::new()
                        .with::<GraphReAct>()
                        .with::<Streaming>(),
                ),
            integrations: vec![server_integration],
        })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use agentc_compiler::generator::context::GenerationContext;

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
        let resolved = ReActGraph.resolve(context(None), ReActGraphConfig::default()).unwrap();

        assert!(resolved.contribution.provides.contains::<GraphReAct>());
        assert!(resolved.contribution.provides.contains::<Streaming>());
    }

    #[test]
    fn core_contribution_has_no_http_dependent_blocks() {
        let resolved = ReActGraph.resolve(context(None), ReActGraphConfig::default()).unwrap();

        let ids = resolved
            .contribution
            .blocks
            .iter()
            .map(|block| block.id().to_string())
            .collect::<Vec<_>>();

        assert_eq!(ids, vec!["agent_rs", "cli_run", "react_migrations", "react_cargo"]);
        assert_eq!(resolved.integrations.len(), 1);
        assert!(
            resolved.integrations[0]
                .contribution
                .requires
                .contains::<HttpServer>()
        );
    }

    #[tokio::test]
    async fn react_cargo_enables_ag_ui_feature_only_when_protocol_present() {
        let without_ag_ui =
            ReActGraph.resolve(context(None), ReActGraphConfig::default()).unwrap();
        let with_ag_ui = ReActGraph
            .resolve(
                context(Some(json!({
                    "host": "0.0.0.0",
                    "port": 8080,
                    "protocols": [{ "type": "ag_ui", "config": { "path": "/ag-ui" } }]
                }))),
                ReActGraphConfig::default(),
            )
            .unwrap();

        let ctx = GenerationContext::new(context(None));

        let without = without_ag_ui
            .contribution
            .blocks
            .iter()
            .find(|b| b.id() == "react_cargo")
            .unwrap()
            .render_contribution(&ctx, "cargo::dependencies")
            .await
            .unwrap();
        let with = with_ag_ui
            .contribution
            .blocks
            .iter()
            .find(|b| b.id() == "react_cargo")
            .unwrap()
            .render_contribution(&ctx, "cargo::dependencies")
            .await
            .unwrap();

        assert!(!without.contains("ag-ui"));
        assert!(with.contains("ag-ui"));
    }
}
