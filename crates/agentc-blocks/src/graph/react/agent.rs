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
    graph::{
        codegen::{
            a2a::A2aCodeGen, identity::IdentityCodeGen, mcp::McpCodeGen,
            models::ModelRegistryCodeGen, prompt::PromptSourceCodeGen,
            skills::SkillsCodeGen, tools::ToolsCodeGen,
        },
        react::ReActGraphConfig,
    },
    types::RuntimeValue,
};

pub struct AgentCodeGen {
    pub fields: FieldsSpec,
    pub config: ReActGraphConfig,
}

impl AgentCodeGen {
    fn push_runtime_value_loader<T>(
        path: &[&str],
        value: &RuntimeValue<T>,
        calls: &mut Vec<TokenStream>,
    ) where
        T: serde::Serialize,
    {
        let path_segments = path.to_vec();

        match value {
            RuntimeValue::Constant(value) => {
                let value = serde_json::to_string(value)
                    .unwrap()
                    .parse::<TokenStream>()
                    .unwrap();

                calls.push(quote! {
                    .constant(
                        path![#(#path_segments),*],
                        serde_json::json!(#value)
                    )
                });
            }
            RuntimeValue::Runtime { default, .. } => {
                if let Some(default) = default {
                    let default = serde_json::to_string(default)
                        .unwrap()
                        .parse::<TokenStream>()
                        .unwrap();

                    calls.push(quote! {
                        .default(
                            path![#(#path_segments),*],
                            serde_json::json!(#default)
                        )
                    });
                }
            }
        }
    }

    fn push_runtime_value_mapper<T>(
        path: &[&str],
        value: &RuntimeValue<T>,
        fields: &mut Vec<TokenStream>,
    ) {
        let path_segments = path.to_vec();

        if let RuntimeValue::Runtime { env, .. } = value {
            fields.push(quote! {
                .field(path![#(#path_segments),*], #env)
            });
        }
    }

    fn config_loader_calls(&self) -> TokenStream {
        let mut calls = Vec::new();

        if let Some(model) = &self.config.model {
            if let Some(timeout) = &model.timeout {
                Self::push_runtime_value_loader(
                    &["react", "model", "timeout"],
                    timeout,
                    &mut calls,
                );
            }

            if let Some(retry) = &model.retry {
                Self::push_runtime_value_loader(
                    &["react", "model", "retry", "max_attempts"],
                    &retry.max_attempts,
                    &mut calls,
                );
                Self::push_runtime_value_loader(
                    &["react", "model", "retry", "initial_backoff"],
                    &retry.initial_backoff,
                    &mut calls,
                );
                Self::push_runtime_value_loader(
                    &["react", "model", "retry", "max_backoff"],
                    &retry.max_backoff,
                    &mut calls,
                );
            }
        }

        quote! { #(#calls)* }
    }

    fn config_mapper_fields(&self) -> TokenStream {
        let mut fields = Vec::new();

        if let Some(model) = &self.config.model {
            if let Some(timeout) = &model.timeout {
                Self::push_runtime_value_mapper(
                    &["react", "model", "timeout"],
                    timeout,
                    &mut fields,
                );
            }

            if let Some(retry) = &model.retry {
                Self::push_runtime_value_mapper(
                    &["react", "model", "retry", "max_attempts"],
                    &retry.max_attempts,
                    &mut fields,
                );
                Self::push_runtime_value_mapper(
                    &["react", "model", "retry", "initial_backoff"],
                    &retry.initial_backoff,
                    &mut fields,
                );
                Self::push_runtime_value_mapper(
                    &["react", "model", "retry", "max_backoff"],
                    &retry.max_backoff,
                    &mut fields,
                );
            }
        }

        quote! { #(#fields)* }
    }
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
        let prompt_source = PromptSourceCodeGen::generate(ctx)?;

        let source = quote! {
            use std::sync::Arc;
            use anyhow::Result;
            use tokio_util::sync::CancellationToken;

            use agentc_database::Database;
            use agentc_prompt::{
                compaction::TailWindow,
                counter::TiktokenCounter,
                source::ConstantPromptSource,
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
                graph::{ReActGraphConfig, ReActNode},
                types::{
                    event::Event,
                    message::Message,
                    model::{ModelConfig, ModelConfigRetry},
                },
            };

            use crate::config::{A2aTenantConfig, Config, McpTransportConfig};

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
                        ReActNode::graph(ReActGraphConfig {
                            default_model_config: ModelConfig::new()
                                .maybe_with_timeout(config.react.model.timeout)
                                .maybe_with_retry(
                                    config
                                        .react
                                        .model
                                        .retry
                                        .as_ref()
                                        .map(|retry| ModelConfigRetry {
                                            max_attempts: retry.max_attempts,
                                            initial_backoff: retry.initial_backoff,
                                            max_backoff: retry.max_backoff,
                                        })
                                ),
                        })
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
                    .with_compaction_strategy(TailWindow)
                    .with_prompt_source(#prompt_source);

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
        };

        Ok(vec![("src/agent.rs".into(), source)])
    }

    fn generate_contribution(
        &self,
        ctx: &GenerationContext<ResolvedContext>,
        point: &str,
    ) -> Result<TokenStream, GeneratorError> {
        match point {
            "config::fields" => Ok(quote! {
                pub react: ReActConfig,
            }),
            "config::impls" => Ok(quote! {
                #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
                #[serde(default)]
                pub struct ReActConfig {
                    pub model: ReActModelConfig,
                }

                #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
                #[serde(default)]
                pub struct ReActModelConfig {
                    pub timeout: Option<u64>,
                    pub retry: Option<ReActModelRetryConfig>,
                }

                #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
                pub struct ReActModelRetryConfig {
                    pub max_attempts: u32,
                    pub initial_backoff: u64,
                    pub max_backoff: u64,
                }
            }),
            "config::loader" => {
                let mcp = McpCodeGen::loader_calls(ctx);
                let a2a = A2aCodeGen::loader_calls(ctx);
                let react = self.config_loader_calls();

                Ok(quote! {
                    #mcp
                    #a2a
                    #react
                })
            }
            "config::mapper" => {
                let mcp = McpCodeGen::mapper_fields(ctx);
                let a2a = A2aCodeGen::mapper_fields(ctx);
                let react = self.config_mapper_fields();

                Ok(quote! {
                    #mcp
                    #a2a
                    #react
                })
            }
            "tools::features" => Ok(ToolsCodeGen::features(ctx)),
            _ => Err(GeneratorError::unexpected(format!("Unknown extension point '{}'", point))),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::graph::{ReActGraphModelConfig, ReActGraphModelRetryConfig};

    struct AgentCodeGenFixture;

    impl AgentCodeGenFixture {
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

        fn generated_agent() -> String {
            AgentCodeGen {
                fields: FieldsSpec::collect_from(&Self::context()),
                config: ReActGraphConfig::default(),
            }
            .generate_files(&GenerationContext::new(Self::context()), &ExtensionRegistry::empty())
            .unwrap()
            .into_iter()
            .find(|(path, _)| path == &PathBuf::from("src/agent.rs"))
            .expect("agent file should be generated")
            .1
            .to_string()
        }

        fn configured_codegen() -> AgentCodeGen {
            AgentCodeGen {
                fields: FieldsSpec::collect_from(&Self::context()),
                config: ReActGraphConfig {
                    model: Some(ReActGraphModelConfig {
                        timeout: Some(RuntimeValue::constant(30000)),
                        retry: Some(ReActGraphModelRetryConfig {
                            max_attempts: RuntimeValue::default_runtime("MODEL_MAX_ATTEMPTS", 3),
                            initial_backoff: RuntimeValue::default_runtime(
                                "MODEL_INITIAL_BACKOFF_MS",
                                100,
                            ),
                            max_backoff: RuntimeValue::constant(5000),
                        }),
                    }),
                },
            }
        }
    }

    #[test]
    fn generated_agent_registers_startup_configured_a2a_agents() {
        let rendered = AgentCodeGenFixture::generated_agent();

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
    fn generated_agent_passes_react_model_defaults_to_graph() {
        let rendered = AgentCodeGenFixture::generated_agent();

        assert!(rendered.contains("ReActNode :: graph (ReActGraphConfig"));
        assert!(rendered.contains("default_model_config : ModelConfig :: new"));
        assert!(rendered.contains("config . react . model . timeout"));
        assert!(rendered.contains("ModelConfigRetry"));
    }

    #[test]
    fn generated_agent_wires_constant_prompt_source() {
        let rendered = AgentCodeGenFixture::generated_agent();

        assert!(rendered.contains("with_prompt_source (ConstantPromptSource :: new"));
        assert!(rendered.contains("PromptTemplate :: default"));
    }

    #[test]
    fn react_model_defaults_contribute_generated_config() {
        let codegen = AgentCodeGenFixture::configured_codegen();
        let context = GenerationContext::new(AgentCodeGenFixture::context());

        let impls = codegen
            .generate_contribution(&context, "config::impls")
            .unwrap()
            .to_string();
        let loader = codegen
            .generate_contribution(&context, "config::loader")
            .unwrap()
            .to_string();
        let mapper = codegen
            .generate_contribution(&context, "config::mapper")
            .unwrap()
            .to_string();

        assert!(
            codegen
                .generate_contribution(&context, "config::fields")
                .unwrap()
                .to_string()
                .contains("react : ReActConfig")
        );
        assert!(impls.contains("struct ReActModelConfig"));
        assert!(impls.contains("struct ReActModelRetryConfig"));
        assert!(loader.contains("\"react\" , \"model\" , \"timeout\""));
        assert!(loader.contains("\"max_attempts\""));
        assert!(loader.contains("\"initial_backoff\""));
        assert!(loader.contains("\"max_backoff\""));
        assert!(mapper.contains("MODEL_MAX_ATTEMPTS"));
        assert!(mapper.contains("MODEL_INITIAL_BACKOFF_MS"));
    }
}
