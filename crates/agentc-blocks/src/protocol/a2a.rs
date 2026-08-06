// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use proc_macro2::TokenStream;
use quote::quote;
use std::path::PathBuf;

use agentc_compiler::generator::{
    blocks::{
        BlockSet,
        codegen::{CodeGen, CodeGenBlock},
        template::{TemplateFragment, TemplateFragmentBlock},
    },
    context::GenerationContext,
    errors::GeneratorError,
    extension::{Contribution, ErasedContributionValue, ExtensionRegistry},
};

use crate::{
    archetype::standalone::codegen::cargo::{CargoDependencyContribution, CargoPatchContribution},
    composition::GenerationContribution,
    context::{ResolvedContext, ResolvedContextHttpServerProtocolA2a},
    contributions::dependency::RuntimeDependencyContribution,
    errors::BlocksError,
    feature::{GenerationFeatureSet, HttpServer, ProtocolA2a, Streaming, SupportsA2a},
    protocol::{traits::Protocol, types::ResolvedProtocol},
};

pub struct A2aCodeGen {
    pub config: ResolvedContextHttpServerProtocolA2a,
}

pub struct A2aCargoFragment;

impl CodeGen<ResolvedContext> for A2aCodeGen {
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
                            .nest(
                                #config_path,
                                agentc_protocol_a2a::router::router(
                                    service.clone(),
                                    agentc_protocol_a2a::protocol::AgentInterface::new(
                                        #config_path,
                                        "HTTP+JSON",
                                    ),
                                    default_tenant_id.clone(),
                                    task_queue.clone(),
                                ),
                            )
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

impl TemplateFragment<ResolvedContext> for A2aCargoFragment {
    fn generate_contribution(
        &self,
        _ctx: &GenerationContext<ResolvedContext>,
        point: &str,
    ) -> Result<ErasedContributionValue, GeneratorError> {
        match point {
            "cargo::dependencies" => {
                Ok(ErasedContributionValue::new(CargoDependencyContribution::runtime(
                    RuntimeDependencyContribution::new("agentc-protocol-a2a")
                        .default_features(false)
                        .feature("server"),
                )))
            }
            "cargo::patches" => Ok(ErasedContributionValue::new(CargoPatchContribution::runtime(
                RuntimeDependencyContribution::new("agentc-protocol-a2a"),
            ))),
            _ => Err(GeneratorError::unexpected(format!("Unknown extension point '{}'", point))),
        }
    }
}

/// The A2A protocol contribution. Requires an archetype-provided `HttpServer`, streaming support,
/// and a graph that implements the A2A adapter.
pub struct A2aProtocol;

impl Protocol for A2aProtocol {
    type Config = ResolvedContextHttpServerProtocolA2a;

    fn name(&self) -> &str {
        "a2a"
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
                                .id("protocol_a2a")
                                .contribute(Contribution::<String>::strict("server::routers"))
                                .build(A2aCodeGen { config }),
                        )
                        .add(
                            TemplateFragmentBlock::builder()
                                .id("protocol_a2a_cargo")
                                .contribute(Contribution::<CargoDependencyContribution>::strict(
                                    "cargo::dependencies",
                                ))
                                .contribute(Contribution::<CargoPatchContribution>::strict(
                                    "cargo::patches",
                                ))
                                .build(A2aCargoFragment),
                        )
                        .into_inner(),
                )
                .with_provides(GenerationFeatureSet::new().with::<ProtocolA2a>())
                .with_requires(
                    GenerationFeatureSet::new()
                        .with::<HttpServer>()
                        .with::<Streaming>()
                        .with::<SupportsA2a>(),
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
        let resolved = A2aProtocol
            .resolve(context(), ResolvedContextHttpServerProtocolA2a { path: "/a2a".to_string() })
            .unwrap();

        assert_eq!(resolved.name, "a2a");
        assert!(
            resolved
                .contribution
                .provides
                .contains::<ProtocolA2a>()
        );
        assert!(
            resolved
                .contribution
                .requires
                .contains::<HttpServer>()
        );
        assert!(
            resolved
                .contribution
                .requires
                .contains::<Streaming>()
        );
        assert!(
            resolved
                .contribution
                .requires
                .contains::<SupportsA2a>()
        );
    }

    #[test]
    fn a2a_codegen_router_contribution_nests_at_configured_path() {
        let codegen = A2aCodeGen {
            config: ResolvedContextHttpServerProtocolA2a { path: "/custom-a2a".to_string() },
        };

        let rendered = codegen
            .generate_contribution(&GenerationContext::new(context()), "server::routers")
            .unwrap()
            .to_string();

        assert!(rendered.contains("custom-a2a"));
        assert!(rendered.contains("agentc_protocol_a2a :: router :: router"));
        assert!(rendered.contains("AgentInterface :: new"));
        assert!(rendered.contains("service . clone"));
        assert!(rendered.contains("default_tenant_id . clone"));
        assert!(rendered.contains("task_queue . clone"));
    }

    #[test]
    fn a2a_cargo_fragment_contributes_server_runtime_dependency() {
        let dependency = A2aCargoFragment
            .generate_contribution(&GenerationContext::new(context()), "cargo::dependencies")
            .unwrap()
            .downcast::<CargoDependencyContribution>()
            .unwrap();

        assert!(matches!(
            dependency,
            CargoDependencyContribution::Runtime(dependency)
                if dependency.name == "agentc-protocol-a2a"
                    && dependency.default_features == Some(false)
                    && dependency.features.len() == 1
                    && dependency.features.contains("server")
        ));
    }

    #[test]
    fn a2a_cargo_fragment_contributes_runtime_patch() {
        let patch = A2aCargoFragment
            .generate_contribution(&GenerationContext::new(context()), "cargo::patches")
            .unwrap()
            .downcast::<CargoPatchContribution>()
            .unwrap();

        assert!(matches!(
            patch,
            CargoPatchContribution::Runtime(dependency)
                if dependency.name == "agentc-protocol-a2a"
        ));
    }
}
