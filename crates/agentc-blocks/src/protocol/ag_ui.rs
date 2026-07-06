// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use proc_macro2::TokenStream;
use quote::quote;
use std::path::PathBuf;

use agentc_compiler::generator::{
    blocks::{BlockSet, codegen::{CodeGen, CodeGenBlock}},
    context::GenerationContext,
    errors::GeneratorError,
    extension::{Contribution, ExtensionRegistry},
};

use crate::{
    composition::GenerationContribution,
    context::{ResolvedContext, ResolvedContextHttpServerProtocolAgUi},
    errors::BlocksError,
    feature::{GenerationFeatureSet, GraphReAct, HttpServer, ProtocolAgUi, Streaming},
    protocol::{traits::Protocol, types::ResolvedProtocol},
};

pub struct AgUiCodeGen {
    pub config: ResolvedContextHttpServerProtocolAgUi,
}

impl CodeGen<ResolvedContext> for AgUiCodeGen {
    fn generate_contribution(
        &self,
        _ctx: &GenerationContext<ResolvedContext>,
        point: &str,
    ) -> Result<TokenStream, GeneratorError> {
        match point {
            "server::routers" => {
                let config_path = &self.config.path;

                Ok(quote! {
                    builder = builder.with_router(
                        utoipa_axum::router::OpenApiRouter::new()
                            .nest(#config_path, agentc_protocol_ag_ui::router::router(state.clone()))
                    );
                })
            }
            _ => Err(GeneratorError::unexpected(format!("Unknown extension point '{}'", point))),
        }
    }

    fn generate_files(
        &self,
        _ctx: &GenerationContext<ResolvedContext>,
        _registry: &ExtensionRegistry,
    ) -> Result<Vec<(PathBuf, TokenStream)>, GeneratorError> {
        Ok(vec![])
    }
}

/// The AG-UI protocol contribution. Requires an archetype-provided `HttpServer` and a
/// `Streaming`- and `GraphReAct`-providing graph, since AG-UI is currently only
/// compatible with the ReAct graph's service and event types.
pub struct AgUiProtocol;

impl Protocol for AgUiProtocol {
    type Config = ResolvedContextHttpServerProtocolAgUi;

    fn name(&self) -> &str {
        "ag_ui"
    }

    fn resolve(
        &self,
        _context: ResolvedContext,
        config: Self::Config,
    ) -> Result<ResolvedProtocol, BlocksError> {
        Ok(ResolvedProtocol {
            name: self.name().to_string(),
            contribution: GenerationContribution::new()
                .with_blocks(
                    BlockSet::new()
                        .add(
                            CodeGenBlock::builder()
                                .id("protocol_ag_ui")
                                .contribute(Contribution::strict("server::routers"))
                                .build(AgUiCodeGen { config }),
                        )
                        .into_inner(),
                )
                .with_provides(GenerationFeatureSet::new().with::<ProtocolAgUi>())
                .with_requires(
                    GenerationFeatureSet::new()
                        .with::<HttpServer>()
                        .with::<Streaming>()
                        .with::<GraphReAct>(),
                ),
        })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn context() -> ResolvedContext {
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
            "http_server": null
        }))
        .unwrap()
    }

    #[test]
    fn resolves_with_expected_provides_and_requires() {
        let resolved = AgUiProtocol
            .resolve(context(), ResolvedContextHttpServerProtocolAgUi { path: "/ag-ui".to_string() })
            .unwrap();

        assert_eq!(resolved.name, "ag_ui");
        assert!(resolved.contribution.provides.contains::<ProtocolAgUi>());
        assert!(resolved.contribution.requires.contains::<HttpServer>());
        assert!(resolved.contribution.requires.contains::<Streaming>());
        assert!(resolved.contribution.requires.contains::<GraphReAct>());
    }

    #[test]
    fn ag_ui_codegen_router_contribution_nests_at_configured_path() {
        let codegen = AgUiCodeGen {
            config: ResolvedContextHttpServerProtocolAgUi { path: "/custom-ag-ui".to_string() },
        };

        let rendered = codegen
            .generate_contribution(&GenerationContext::new(context()), "server::routers")
            .unwrap()
            .to_string();

        assert!(rendered.contains("custom-ag-ui"));
        assert!(rendered.contains("agentc_protocol_ag_ui :: router :: router"));
    }
}
