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
        identity::IdentityCodeGen, mcp::McpCodeGen, models::ModelRegistryCodeGen,
        skills::SkillsCodeGen, tools::ToolsCodeGen,
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
            use agentc_agent_react::{
                checkpoint::handle::SqlReActCheckpointStoreHandle,
                graph::ReActNode,
                types::{
                    event::Event,
                    message::Message,
                },
            };

            use crate::config::{Config, McpTransportConfig};

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
                                    SqlReActCheckpointStoreHandle::new(db)
                                )
                            )
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

                Ok(
                    builder
                        .with_identity(#agent_identity)
                        .build()?
                )
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
            "config::loader" => Ok(McpCodeGen::loader_calls(ctx)),
            "config::mapper" => Ok(McpCodeGen::mapper_fields(ctx)),
            "tools::features" => Ok(ToolsCodeGen::features(ctx)),
            _ => Err(GeneratorError::unexpected(format!("Unknown extension point '{}'", point))),
        }
    }
}
