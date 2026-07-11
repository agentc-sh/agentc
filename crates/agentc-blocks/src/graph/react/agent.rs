// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use proc_macro2::TokenStream;
use quote::quote;
use std::path::PathBuf;

use agentc_compiler::generator::{
    blocks::codegen::CodeGen, context::GenerationContext, errors::GeneratorError,
    extension::ExtensionRegistry,
};

use crate::{
    context::ResolvedContext,
    fields::FieldsSpec,
    graph::codegen::{
        a2a::A2aCodeGen, identity::IdentityCodeGen, mcp::McpCodeGen,
        models::ModelRegistryCodeGen, skills::SkillsCodeGen, tools::ToolsCodeGen,
    },
};

pub struct AgentCodeGen {
    pub fields: FieldsSpec,
}

impl CodeGen<ResolvedContext> for AgentCodeGen {
    fn generate_files(
        &self,
        ctx: &GenerationContext<ResolvedContext>,
        registry: &ExtensionRegistry,
    ) -> Result<Vec<(PathBuf, TokenStream)>, GeneratorError> {
        let extra_use = registry
            .get("agent::use")
            .and_then(|s| s.parse::<TokenStream>().ok());

        let extra_tools = registry
            .get("agent::tools")
            .and_then(|s| s.parse::<TokenStream>().ok());

        let (model_imports, model_registrations) =
            ModelRegistryCodeGen::generate(ctx, &self.fields)?;
        let (tool_imports, tool_registrations) = ToolsCodeGen::generate(ctx, &self.fields)?;
        let (skill_imports, skill_registrations) = SkillsCodeGen::generate(ctx)?;
        let agent_identity = IdentityCodeGen::generate(ctx, &self.fields)?;

        let source = quote! {
            use std::sync::Arc;
            use anyhow::Result;
            use tokio_util::sync::CancellationToken;

            use agentc_database::Database;
            use agentc_prompt::{
                compaction::TailWindow,
                counter::TiktokenCounter,
                template::{PromptTemplate, Role},
            };
            use agentc_model::registry::ModelRegistry;
            use agentc_agent::{
                agent::Agent,
                graph::checkpoint::GraphCheckpointer,
                types::{
                    identity::AgentIdentity,
                    capability::{CapabilitySet, CapabilityPolicy},
                },
            };
            use agentc_mcp::{
                builder::AgentBuilderMcpExt,
                config::{McpServerConfig, McpTransport},
                registry::McpRegistry,
            };
            use agentc_protocol_a2a::{
                client::{A2aClient, A2aClientConfig},
                tools::{A2aTenantPolicy, A2aToolTarget},
            };
            use agentc_agent_react::{
                cancel::SqlReActCanceller,
                checkpoint::handle::SqlReActCheckpointStoreHandle,
                graph::ReActNode,
                types::{
                    event::Event,
                    message::Message,
                },
            };

            use crate::config::{A2aAgentConfig, A2aTenantConfig, Config, McpTransportConfig};

            #(#model_imports)*
            #(#tool_imports)*
            #(#skill_imports)*

            #extra_use

            pub async fn build_agent(
                db: Arc<Database>,
                config: &Config,
                shutdown: CancellationToken,
            ) -> Result<Agent<ReActNode, Event, Message>> {
                let model_registry = ModelRegistry::builder()
                    #(#model_registrations)*
                    .build();

                let mut builder = Agent::builder()
                    .with_graph(
                        ReActNode::graph()
                            .with_checkpointer(
                                GraphCheckpointer::new(
                                    SqlReActCheckpointStoreHandle::new(db.clone())
                                )
                            )
                            .with_canceller(SqlReActCanceller::new(db))
                            .build()
                    )
                    .with_model_registry(model_registry)
                    .with_token_counter(TiktokenCounter::o200k_base())
                    .with_compaction_strategy(TailWindow);

                #(#tool_registrations)*
                #(#skill_registrations)*

                #extra_tools

                if !config.mcp.servers.is_empty() {
                    let mut mcp_builder = McpRegistry::builder();

                    for (name, transport) in &config.mcp.servers {
                        mcp_builder = mcp_builder.with_server(
                            McpServerConfig::new(name.clone(), match transport {
                                McpTransportConfig::Stdio { command, args, env } => McpTransport::Stdio {
                                    command: command.clone(),
                                    args: args.clone(),
                                    env: env.clone(),
                                },
                                McpTransportConfig::Http { url, auth_token, headers } => McpTransport::StreamableHttp {
                                    url: url.clone(),
                                    auth_token: auth_token.clone(),
                                    headers: headers.clone(),
                                },
                            })
                        );
                    }

                    builder = builder.with_mcp_registry(&mcp_builder.build().await?).await;
                }

                for (name, agent) in &config.a2a.agents {
                    if !agent.enabled {
                        continue;
                    }

                    let mut client_config = A2aClientConfig::new(agent.url.clone())
                        .timeout(std::time::Duration::from_secs(agent.timeout_secs));
                    client_config.default_headers = build_a2a_headers(agent)?;

                    let target = A2aToolTarget::builder()
                        .id(name)
                        .name(agent.description.as_deref().unwrap_or(name))
                        .client(A2aClient::new(client_config)?)
                        .tenant_policy(match &agent.tenant {
                            A2aTenantConfig::Inherit => A2aTenantPolicy::Inherit,
                            A2aTenantConfig::None => A2aTenantPolicy::None,
                            A2aTenantConfig::Fixed { id } => A2aTenantPolicy::Fixed(id.clone()),
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

                Ok(
                    builder
                        .with_identity(#agent_identity)
                        .build()?
                )
            }

            fn build_a2a_headers(agent: &A2aAgentConfig) -> Result<reqwest::header::HeaderMap> {
                let mut headers = reqwest::header::HeaderMap::new();

                if let Some(token) = &agent.auth_token {
                    headers.insert(
                        reqwest::header::AUTHORIZATION,
                        reqwest::header::HeaderValue::from_str(&format!("Bearer {token}"))?,
                    );
                }

                for (key, value) in &agent.headers {
                    headers.insert(
                        reqwest::header::HeaderName::from_bytes(key.as_bytes())?,
                        reqwest::header::HeaderValue::from_str(value)?,
                    );
                }

                Ok(headers)
            }
        };

        Ok(vec![("src/agent.rs".into(), source)])
    }

    fn generate_contribution(
        &self,
        ctx: &GenerationContext<ResolvedContext>,
        point: &str,
    ) -> Result<TokenStream, GeneratorError> {
        match point {
            "config::loader" => {
                let mcp = McpCodeGen::loader_calls(ctx);
                let a2a = A2aCodeGen::loader_calls(ctx);

                Ok(quote! {
                    #mcp
                    #a2a
                })
            }
            "config::mapper" => {
                let mcp = McpCodeGen::mapper_fields(ctx);
                let a2a = A2aCodeGen::mapper_fields(ctx);

                Ok(quote! {
                    #mcp
                    #a2a
                })
            }
            "tools::features" => Ok(ToolsCodeGen::features(ctx)),
            _ => Err(GeneratorError::unexpected(format!("Unknown extension point '{}'", point))),
        }
    }
}
