// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use proc_macro2::TokenStream;
use quote::quote;
use std::path::PathBuf;

use agentc_compiler::generator::{
    blocks::codegen::CodeGen,
    context::GenerationContext,
    errors::GeneratorError,
    extension::{ErasedContributionValue, ExtensionRegistry, RenderedTokenStream},
};

use crate::{
    config::fields::FieldsSpec,
    context::ResolvedContext,
    graph::{
        codegen::{
            identity::IdentityCodeGen, models::ModelRegistryCodeGen, prompt::PromptSourceCodeGen,
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
        let (prompt_imports, prompt_source) = PromptSourceCodeGen::generate(ctx, &self.fields)?;

        let source = quote! {
            use std::sync::Arc;
            use anyhow::Result;
            use tokio_util::sync::CancellationToken;

            use agentc_database::Database;
            use agentc_fs::Fs;
            use agentc_http::client::HttpClient;
            use agentc_prompt::{
                compaction::TailWindow,
                counter::TiktokenCounter,
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

            use crate::config::Config;

            #prompt_imports
            #(#model_imports)*
            #(#tool_imports)*
            #(#skill_imports)*

            #extra_use

            pub async fn build_agent(
                db: Arc<Database>,
                fs: Fs,
                http: HttpClient,
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
    ) -> Result<ErasedContributionValue, GeneratorError> {
        match point {
            "config::fields" => {
                Ok(ErasedContributionValue::new(RenderedTokenStream::from(quote! {
                    pub react: ConfigReAct,
                })))
            }
            "config::impls" => {
                Ok(ErasedContributionValue::new(RenderedTokenStream::from(quote! {
                    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
                    #[serde(default)]
                    pub struct ConfigReAct {
                        pub model: ConfigReActModel,
                    }

                    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
                    #[serde(default)]
                    pub struct ConfigReActModel {
                        pub timeout: Option<u64>,
                        pub retry: Option<ConfigReActModelRetry>,
                    }

                    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
                    pub struct ConfigReActModelRetry {
                        pub max_attempts: u32,
                        pub initial_backoff: u64,
                        pub max_backoff: u64,
                    }
                })))
            }
            "config::loader" => Ok(ErasedContributionValue::new(RenderedTokenStream::from(
                self.config_loader_calls(),
            ))),
            "config::mapper" => Ok(ErasedContributionValue::new(RenderedTokenStream::from(
                self.config_mapper_fields(),
            ))),
            "tools::features" => {
                Ok(ErasedContributionValue::new(ToolsCodeGen::features(ctx).to_string()))
            }
            _ => Err(GeneratorError::unexpected(format!("Unknown extension point '{}'", point))),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::{
        context::{
            ResolvedContextAgentPromptSource,
            ResolvedContextAgentPromptSourceLangfuse,
            ResolvedContextTool,
            ResolvedContextToolBash,
            ResolvedContextToolBashEnv,
            ResolvedContextToolBashLimits,
            ResolvedContextToolKind,
        },
        graph::{ReActGraphModelConfig, ReActGraphModelRetryConfig},
    };

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

        fn bash_context() -> ResolvedContext {
            let mut context = Self::context();

            context.tools.insert(
                String::from("shell"),
                ResolvedContextTool {
                    name: String::from("shell"),
                    description: None,
                    enabled: RuntimeValue::constant(true),
                    capabilities: Vec::new(),
                    config: Default::default(),
                    kind: ResolvedContextToolKind::Bash(ResolvedContextToolBash {
                        commands: vec![String::from("git")],
                        cwd: String::from("/workspace"),
                        env: ResolvedContextToolBashEnv::Empty,
                        limits: ResolvedContextToolBashLimits {
                            max_execution_time_secs: 7,
                            max_output_size: 512,
                            max_command_count: 23,
                            max_loop_iterations: 29,
                        },
                        shared: true,
                    }),
                },
            );

            context
        }

        fn generated_agent() -> String {
            Self::generated_agent_for(Self::context())
        }

        fn generated_agent_for(context: ResolvedContext) -> String {
            AgentCodeGen {
                fields: FieldsSpec::collect_from(&context),
                config: ReActGraphConfig::default(),
            }
            .generate_files(&GenerationContext::new(context), &ExtensionRegistry::empty())
            .unwrap()
            .into_iter()
            .find(|(path, _)| path == &PathBuf::from("src/agent.rs"))
            .expect("agent file should be generated")
            .1
            .to_string()
        }

        fn langfuse_context() -> ResolvedContext {
            let mut context = Self::context();

            context.agent.prompt = Some(ResolvedContextAgentPromptSource::Langfuse(
                ResolvedContextAgentPromptSourceLangfuse {
                    prompt_name: RuntimeValue::constant("support/assistant".to_string()),
                    public_key: RuntimeValue::required_runtime("LANGFUSE_PUBLIC_KEY"),
                    secret_key: RuntimeValue::secret_runtime("LANGFUSE_SECRET_KEY"),
                    base_url: Some(RuntimeValue::constant(
                        "https://cloud.langfuse.com".to_string(),
                    )),
                    label: Some(RuntimeValue::constant("staging".to_string())),
                    version: None,
                    cache_ttl_seconds: Some(RuntimeValue::constant(30)),
                    fetch_timeout_seconds: Some(RuntimeValue::constant(5)),
                    max_retries: Some(RuntimeValue::constant(2)),
                },
            ));

            context
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
    fn generated_agent_names_no_mcp_or_a2a_wiring() {
        let rendered = AgentCodeGenFixture::generated_agent();

        assert!(!rendered.contains("config . a2a . agents"));
        assert!(!rendered.contains("config . mcp . servers"));
        assert!(!rendered.contains("agentc_protocol_a2a"));
        assert!(!rendered.contains("agentc_mcp"));
    }

    #[test]
    fn generated_agent_threads_process_resources() {
        let rendered = AgentCodeGenFixture::generated_agent();

        assert!(rendered.contains("use agentc_fs :: Fs"));
        assert!(rendered.contains("use agentc_http :: client :: HttpClient"));
        assert!(rendered.contains("db : Arc < Database >"));
        assert!(rendered.contains("fs : Fs"));
        assert!(rendered.contains("http : HttpClient"));
    }

    #[test]
    fn generated_agent_registers_bash_with_process_resources() {
        let rendered =
            AgentCodeGenFixture::generated_agent_for(AgentCodeGenFixture::bash_context());

        assert!(rendered.contains("BashTool :: builder (fs . clone () , http . clone ())"));
        assert!(rendered.contains("shared ()"));
        assert!(!rendered.contains("FsPolicy"));
        assert!(!rendered.contains("NetworkPolicy"));
        assert!(!rendered.contains("fs_policy"));
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
    fn generated_agent_wires_langfuse_prompt_source() {
        let rendered =
            AgentCodeGenFixture::generated_agent_for(AgentCodeGenFixture::langfuse_context());

        assert!(rendered.contains("use std :: time :: Duration"));
        assert!(rendered.contains("LangfusePromptSource"));
        assert!(rendered.contains("LangfuseClient"));
        assert!(rendered.contains("LangfusePromptSource :: builder"));
        assert!(rendered.contains("LangfuseClient :: builder"));
        assert!(rendered.contains("config . agent . prompt . langfuse . prompt_name"));
        assert!(rendered.contains("config . agent . prompt . langfuse . public_key"));
        assert!(rendered.contains("config . agent . prompt . langfuse . secret_key"));
        assert!(rendered.contains("base_url"));
        assert!(rendered.contains("fetch_timeout"));
        assert!(rendered.contains("max_retries"));
        assert!(rendered.contains("label"));
        assert!(rendered.contains("cache_ttl"));
    }

    #[test]
    fn react_model_defaults_contribute_generated_config() {
        let codegen = AgentCodeGenFixture::configured_codegen();
        let context = GenerationContext::new(AgentCodeGenFixture::context());

        let impls = codegen
            .generate_contribution(&context, "config::impls")
            .unwrap()
            .downcast::<RenderedTokenStream>()
            .unwrap()
            .as_str()
            .to_string();
        let loader = codegen
            .generate_contribution(&context, "config::loader")
            .unwrap()
            .downcast::<RenderedTokenStream>()
            .unwrap()
            .as_str()
            .to_string();
        let mapper = codegen
            .generate_contribution(&context, "config::mapper")
            .unwrap()
            .downcast::<RenderedTokenStream>()
            .unwrap()
            .as_str()
            .to_string();

        assert!(
            codegen
                .generate_contribution(&context, "config::fields")
                .unwrap()
                .downcast::<RenderedTokenStream>()
                .unwrap()
                .as_str()
                .contains("react : ConfigReAct")
        );
        assert!(impls.contains("struct ConfigReActModel"));
        assert!(impls.contains("struct ConfigReActModelRetry"));
        assert!(loader.contains("\"react\" , \"model\" , \"timeout\""));
        assert!(loader.contains("\"max_attempts\""));
        assert!(loader.contains("\"initial_backoff\""));
        assert!(loader.contains("\"max_backoff\""));
        assert!(mapper.contains("MODEL_MAX_ATTEMPTS"));
        assert!(mapper.contains("MODEL_INITIAL_BACKOFF_MS"));
    }
}
