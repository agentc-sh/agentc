// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use quote::quote;

use agentc_compiler::generator::{
    blocks::fragment::Fragment,
    context::GenerationContext,
    errors::GeneratorError,
    extension::{ErasedContributionValue, RenderedTokenStream},
};

use crate::{
    context::ResolvedContext,
    contributions::{
        dependency::{
            CargoDependencies, CargoDependencyContribution, CargoPatchContribution, CargoPatches,
            RuntimeDependencyContribution,
        },
        import::{ImportContribution, Imports},
    },
};

pub struct A2aAgentFragment;

impl Fragment<ResolvedContext> for A2aAgentFragment {
    fn generate_contribution(
        &self,
        _ctx: &GenerationContext<ResolvedContext>,
        point: &str,
    ) -> Result<ErasedContributionValue, GeneratorError> {
        match point {
            "agent::use" => Ok(ErasedContributionValue::new(
                Imports::from_entries([
                    ImportContribution::path(&["agentc_protocol_a2a", "client"])
                        .item("A2aClient")
                        .item("A2aClientConfig"),
                    ImportContribution::path(&["agentc_protocol_a2a", "tools"])
                        .item("A2aTenantPolicy")
                        .item("A2aToolTarget"),
                    ImportContribution::path(&["crate", "config"]).item("ConfigA2aAgentTenant"),
                ])
                .map_err(|error| GeneratorError::unexpected(error.to_string()))?,
            )),
            "agent::tools" => Ok(ErasedContributionValue::new(RenderedTokenStream::from(quote! {
                for (name, agent) in &config.a2a.agents {
                    if !agent.enabled {
                        continue;
                    }

                    let mut client_config = A2aClientConfig::new(agent.url.clone())
                        .timeout(std::time::Duration::from_secs(agent.timeout_secs));

                    if let Some(token) = &agent.auth_token {
                        client_config = client_config.try_header(
                            "Authorization",
                            format!("Bearer {token}"),
                        )?;
                    }

                    for (key, value) in &agent.headers {
                        client_config = client_config.try_header(key, value)?;
                    }

                    let target = A2aToolTarget::builder()
                        .id(name)
                        .name(agent.description.as_deref().unwrap_or(name))
                        .client(A2aClient::new(client_config)?)
                        .tenant_policy(match &agent.tenant {
                            ConfigA2aAgentTenant::Inherit => A2aTenantPolicy::Inherit,
                            ConfigA2aAgentTenant::None => A2aTenantPolicy::None,
                            ConfigA2aAgentTenant::Fixed { id } => A2aTenantPolicy::Fixed(id.clone()),
                        })
                        .capabilities(agent.capabilities.clone())
                        .default_accepted_output_modes(agent.default_accepted_output_modes.clone())
                        .build()?;

                    builder = builder
                        .with_typed_tool(target.send_task_tool())
                        .with_typed_tool(target.stream_task_tool())
                        .with_typed_tool(target.get_task_tool())
                        .with_typed_tool(target.cancel_task_tool());
                }
            }))),
            "cargo::dependencies" => Ok(ErasedContributionValue::new(
                CargoDependencies::from_entries([CargoDependencyContribution::runtime(
                    RuntimeDependencyContribution::new("agentc-protocol-a2a")
                        .default_features(false)
                        .feature("client"),
                )])
                .map_err(|error| GeneratorError::unexpected(error.to_string()))?,
            )),
            "cargo::patches" => Ok(ErasedContributionValue::new(
                CargoPatches::from_entries([CargoPatchContribution::runtime(
                    RuntimeDependencyContribution::new("agentc-protocol-a2a"),
                )])
                .map_err(|error| GeneratorError::unexpected(error.to_string()))?,
            )),
            _ => Err(GeneratorError::unexpected(format!("Unknown extension point '{}'", point))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use serde_json::json;

    fn context() -> GenerationContext<ResolvedContext> {
        GenerationContext::new(
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
            .unwrap(),
        )
    }

    fn contribution(point: &str) -> String {
        A2aAgentFragment
            .generate_contribution(&context(), point)
            .unwrap()
            .downcast::<RenderedTokenStream>()
            .unwrap()
            .as_str()
            .to_string()
    }

    #[test]
    fn registers_startup_configured_a2a_agents() {
        let rendered = contribution("agent::tools");

        assert!(rendered.contains("config . a2a . agents"));
        assert!(rendered.contains("A2aClientConfig :: new"));
        assert!(rendered.contains("client_config . try_header"));
        assert!(rendered.contains("target . send_task_tool"));
        assert!(rendered.contains("target . stream_task_tool"));
        assert!(rendered.contains("target . get_task_tool"));
        assert!(rendered.contains("target . cancel_task_tool"));
        assert!(!rendered.contains("build_a2a_headers"));
        assert!(!rendered.contains("reqwest :: header"));
    }

    #[test]
    fn contributes_the_a2a_client_dependency_without_a_declared_a2a_tool() {
        let dependencies = A2aAgentFragment
            .generate_contribution(&context(), "cargo::dependencies")
            .unwrap()
            .downcast::<CargoDependencies>()
            .unwrap();

        assert_eq!(dependencies.len(), 1);
        assert!(matches!(
            dependencies
                .get(&"agentc-protocol-a2a")
                .unwrap(),
            CargoDependencyContribution::Runtime(dependency)
                if dependency.default_features == Some(false)
                    && dependency.features.len() == 1
                    && dependency.features.contains("client")
        ));
    }

    #[test]
    fn contributes_the_a2a_runtime_patch() {
        let patches = A2aAgentFragment
            .generate_contribution(&context(), "cargo::patches")
            .unwrap()
            .downcast::<CargoPatches>()
            .unwrap();

        assert_eq!(patches.len(), 1);
        assert!(
            patches
                .get(&"agentc-protocol-a2a")
                .is_some()
        );
    }
}
