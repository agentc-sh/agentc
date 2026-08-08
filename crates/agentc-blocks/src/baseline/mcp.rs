// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use quote::quote;

use agentc_compiler::generator::{
    blocks::fragment::Fragment, context::GenerationContext, errors::GeneratorError,
    extension::ErasedContributionValue,
};

use crate::{
    context::ResolvedContext,
    contributions::dependency::{
        CargoDependencies, CargoDependencyContribution, CargoPatchContribution, CargoPatches,
        RuntimeDependencyContribution,
    },
};

pub struct McpAgentFragment;

impl Fragment<ResolvedContext> for McpAgentFragment {
    fn generate_contribution(
        &self,
        _ctx: &GenerationContext<ResolvedContext>,
        point: &str,
    ) -> Result<ErasedContributionValue, GeneratorError> {
        match point {
            "agent::use" => Ok(ErasedContributionValue::new(
                quote! {
                    use agentc_mcp::{
                        builder::AgentBuilderMcpExt,
                        config::{McpServerConfig, McpTransport},
                        registry::McpRegistry,
                    };

                    use crate::config::ConfigMcpTransport;
                }
                .to_string(),
            )),
            "agent::tools" => Ok(ErasedContributionValue::new(
                quote! {
                    if !config.mcp.servers.is_empty() {
                        let mut mcp_builder = McpRegistry::builder();

                        for (name, transport) in &config.mcp.servers {
                            mcp_builder = mcp_builder.with_server(
                                McpServerConfig::new(name.clone(), match transport {
                                    ConfigMcpTransport::Stdio { command, args, env } => McpTransport::Stdio {
                                        command: command.clone(),
                                        args: args.clone(),
                                        env: env.clone(),
                                    },
                                    ConfigMcpTransport::Http { url, auth_token, headers } => McpTransport::StreamableHttp {
                                        url: url.clone(),
                                        auth_token: auth_token.clone(),
                                        headers: headers.clone(),
                                    },
                                })
                            );
                        }

                        builder = builder.with_mcp_registry(&mcp_builder.build().await?).await;
                    }
                }
                .to_string(),
            )),
            "cargo::dependencies" => Ok(ErasedContributionValue::new(
                CargoDependencies::from_entries([CargoDependencyContribution::runtime(
                    RuntimeDependencyContribution::new("agentc-mcp"),
                )])
                .map_err(|error| GeneratorError::unexpected(error.to_string()))?,
            )),
            "cargo::patches" => Ok(ErasedContributionValue::new(
                CargoPatches::from_entries([CargoPatchContribution::runtime(
                    RuntimeDependencyContribution::new("agentc-mcp"),
                )])
                .map_err(|error| GeneratorError::unexpected(error.to_string()))?,
            )),
            _ => Err(GeneratorError::unexpected(format!("Unknown extension point '{}'", point))),
        }
    }
}
