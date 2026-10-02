// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

pub mod agent;
pub mod block;
pub mod build;
pub mod errors;
pub mod filesystem;
pub mod graph;
pub mod http_server;
pub mod interpolate;
pub mod network;
pub mod provider;
pub mod runtime;
pub mod skill;
pub mod tool;

pub use agent::*;
pub use block::*;
pub use build::*;
pub use filesystem::*;
pub use graph::*;
pub use http_server::*;
pub use network::*;
pub use provider::*;
pub use runtime::*;
pub use skill::*;
pub use tool::*;

use sanitizer::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use validator::Validate;

use agentc_blocks::{context::*, types::RuntimeValue};
use agentc_compiler::{
    asset::types::{AssetOrigin, AssetRef},
    generator::loader::ResourceLoader,
    transformer::types::TransformedAsset,
};

use crate::manifest::{errors::ManifestError, interpolate::Interpolate};

#[derive(Debug, Clone, Serialize, Deserialize, Validate, Sanitizer)]
pub struct Manifest {
    /// Build configuration for this agent.
    #[serde(default)]
    pub build: ManifestBuild,
    /// Configuration for the runtime environment not specific to any component.
    #[serde(default)]
    pub runtime: ManifestRuntime,
    /// Provider-specific configuration that can be referenced in agent definitions.
    pub providers: ManifestProvider,
    /// Agent definition. Exactly one entry is expected here,
    /// but may support multiple agents in the future.
    #[serde(default)]
    #[validate(nested)]
    pub agent: HashMap<String, ManifestAgent>,
    /// Tools that can be invoked by this agent.
    #[serde(default)]
    #[validate(nested)]
    pub tool: HashMap<String, ManifestTool>,
    /// Skills available to this agent. Each entry may be an embedded directory
    /// or an inlined skill body.
    #[serde(default)]
    #[validate(nested)]
    pub skill: HashMap<String, ManifestSkill>,
    /// Local variables that can be referenced in block templates as `{{ locals.<key> }}`.
    #[serde(default)]
    pub locals: HashMap<String, RuntimeValue<String>>,
    /// Custom blocks defined in this manifest that will be rendered alongside the template's
    /// predefined blocks.
    #[serde(default)]
    #[validate(nested)]
    pub block: HashMap<String, ManifestBlock>,
    /// Optional configuration for an HTTP server to run alongside the agent.
    #[serde(default)]
    #[validate(nested)]
    pub http_server: Option<ManifestHttpServer>,
    /// Outbound network configuration.
    #[serde(default)]
    #[validate(nested)]
    pub network: ManifestNetwork,
    /// Virtual filesystem topology.
    #[serde(default)]
    #[validate(nested)]
    pub filesystem: ManifestFilesystem,
}

impl Manifest {
    fn resolve_locals(&self) -> Value {
        json!({
            "locals": Value::Object(
                self.locals
                    .iter()
                    .filter_map(|(key, value)| {
                        value
                            .default_value()
                            .map(|default| (key.clone(), Value::String(default.to_string())))
                    })
                    .collect()
            )
        })
    }

    fn resolve_agent_label(&self) -> Result<String, ManifestError> {
        let agent_label = self
            .agent
            .iter()
            .next()
            .ok_or_else(|| ManifestError::resolution("manifest must contain at least on `agent`"))
            .map(|(label, _)| label.clone())?;

        if agent_label.is_empty() {
            return Err(ManifestError::resolution("agent label cannot be empty"));
        }

        Ok(agent_label)
    }

    fn resolve_runtime(&self, locals: &Value) -> Result<ResolvedContextRuntime, ManifestError> {
        Ok(ResolvedContextRuntime {
            default_tenant_id: self
                .runtime
                .default_tenant_id
                .clone()
                .interpolate(locals)?,
        })
    }

    fn resolve_provider_params(
        params: ManifestProviderParams,
        locals: &Value,
    ) -> Result<ResolvedContextProviderParams, ManifestError> {
        Ok(ResolvedContextProviderParams {
            max_tokens: params.max_tokens,
            temperature: params.temperature,
            top_p: params.top_p,
            top_k: params.top_k,
            stop_sequences: params
                .stop_sequences
                .interpolate(locals)?,
            frequency_penalty: params.frequency_penalty,
            presence_penalty: params.presence_penalty,
            seed: params.seed,
            provider_params: params
                .provider_params
                .interpolate(locals)?,
        })
    }

    fn resolve_providers(
        &self,
        locals: &Value,
    ) -> Result<Vec<ResolvedContextProvider>, ManifestError> {
        let mut providers = Vec::new();

        if let Some(anthropic) = &self.providers.anthropic {
            providers.push(ResolvedContextProvider::Anthropic(ResolvedContextProviderAnthropic {
                models: anthropic
                    .models
                    .as_ref()
                    .map(|models| {
                        models
                            .iter()
                            .map(|model| match model {
                                ManifestProviderAnthropicModel::Name(name) => {
                                    Ok(ResolvedContextProviderAnthropicModel {
                                        name: name.clone().interpolate(locals)?,
                                        params: None,
                                    })
                                }
                                ManifestProviderAnthropicModel::Config(config) => {
                                    Ok(ResolvedContextProviderAnthropicModel {
                                        name: config
                                            .name
                                            .clone()
                                            .interpolate(locals)?,
                                        params: config
                                            .params
                                            .clone()
                                            .map(|params| {
                                                Self::resolve_provider_params(params, locals)
                                            })
                                            .transpose()?,
                                    })
                                }
                            })
                            .collect::<Result<Vec<_>, ManifestError>>()
                    })
                    .transpose()?,
                config: anthropic
                    .config
                    .as_ref()
                    .map(|config| {
                        Ok::<_, ManifestError>(ResolvedContextProviderAnthropicConfig {
                            api_key: config
                                .api_key
                                .clone()
                                .interpolate(locals)?,
                            base_url: config
                                .base_url
                                .clone()
                                .interpolate(locals)?,
                        })
                    })
                    .transpose()?,
                params: anthropic
                    .params
                    .clone()
                    .map(|params| Self::resolve_provider_params(params, locals))
                    .transpose()?,
            }));
        }

        if let Some(openai) = &self.providers.openai {
            providers.push(ResolvedContextProvider::OpenAi(ResolvedContextProviderOpenAi {
                models: openai
                    .models
                    .as_ref()
                    .map(|models| {
                        models
                            .iter()
                            .map(|model| match model {
                                ManifestProviderOpenAiModel::Name(name) => {
                                    Ok(ResolvedContextProviderOpenAiModel {
                                        name: name.clone().interpolate(locals)?,
                                        params: None,
                                    })
                                }
                                ManifestProviderOpenAiModel::Config(config) => {
                                    Ok(ResolvedContextProviderOpenAiModel {
                                        name: config
                                            .name
                                            .clone()
                                            .interpolate(locals)?,
                                        params: config
                                            .params
                                            .clone()
                                            .map(|params| {
                                                Self::resolve_provider_params(params, locals)
                                            })
                                            .transpose()?,
                                    })
                                }
                            })
                            .collect::<Result<Vec<_>, ManifestError>>()
                    })
                    .transpose()?,
                config: openai
                    .config
                    .as_ref()
                    .map(|config| {
                        Ok::<_, ManifestError>(ResolvedContextProviderOpenAiConfig {
                            api_key: config
                                .api_key
                                .clone()
                                .interpolate(locals)?,
                            base_url: config
                                .base_url
                                .clone()
                                .interpolate(locals)?,
                        })
                    })
                    .transpose()?,
                params: openai
                    .params
                    .clone()
                    .map(|params| Self::resolve_provider_params(params, locals))
                    .transpose()?,
            }));
        }

        if let Some(ollama) = &self.providers.ollama {
            providers.push(ResolvedContextProvider::Ollama(ResolvedContextProviderOllama {
                models: ollama
                    .models
                    .as_ref()
                    .map(|models| {
                        models
                            .iter()
                            .map(|model| match model {
                                ManifestProviderOllamaModel::Name(name) => {
                                    Ok(ResolvedContextProviderOllamaModel {
                                        name: name.clone().interpolate(locals)?,
                                        params: None,
                                    })
                                }
                                ManifestProviderOllamaModel::Config(config) => {
                                    Ok(ResolvedContextProviderOllamaModel {
                                        name: config
                                            .name
                                            .clone()
                                            .interpolate(locals)?,
                                        params: config
                                            .params
                                            .clone()
                                            .map(|params| {
                                                Self::resolve_provider_params(params, locals)
                                            })
                                            .transpose()?,
                                    })
                                }
                            })
                            .collect::<Result<Vec<_>, ManifestError>>()
                    })
                    .transpose()?,
                config: ollama
                    .config
                    .as_ref()
                    .map(|config| {
                        Ok::<_, ManifestError>(ResolvedContextProviderOllamaConfig {
                            base_url: config
                                .base_url
                                .clone()
                                .interpolate(locals)?,
                        })
                    })
                    .transpose()?,
                params: ollama
                    .params
                    .clone()
                    .map(|params| Self::resolve_provider_params(params, locals))
                    .transpose()?,
            }));
        }

        if let Some(openrouter) = &self.providers.openrouter {
            providers.push(ResolvedContextProvider::OpenRouter(
                ResolvedContextProviderOpenRouter {
                    models: openrouter
                        .models
                        .as_ref()
                        .map(|models| {
                            models
                                .iter()
                                .map(|model| match model {
                                    ManifestProviderOpenRouterModel::Name(name) => {
                                        Ok(ResolvedContextProviderOpenRouterModel {
                                            name: name.clone().interpolate(locals)?,
                                            params: None,
                                        })
                                    }
                                    ManifestProviderOpenRouterModel::Config(config) => {
                                        Ok(ResolvedContextProviderOpenRouterModel {
                                            name: config
                                                .name
                                                .clone()
                                                .interpolate(locals)?,
                                            params: config
                                                .params
                                                .clone()
                                                .map(|params| {
                                                    Self::resolve_provider_params(params, locals)
                                                })
                                                .transpose()?,
                                        })
                                    }
                                })
                                .collect::<Result<Vec<_>, ManifestError>>()
                        })
                        .transpose()?,
                    config: openrouter
                        .config
                        .as_ref()
                        .map(|config| {
                            Ok::<_, ManifestError>(ResolvedContextProviderOpenRouterConfig {
                                api_key: config
                                    .api_key
                                    .clone()
                                    .interpolate(locals)?,
                            })
                        })
                        .transpose()?,
                    params: openrouter
                        .params
                        .clone()
                        .map(|params| Self::resolve_provider_params(params, locals))
                        .transpose()?,
                },
            ));
        }

        if let Some(xai) = &self.providers.xai {
            providers.push(ResolvedContextProvider::Xai(ResolvedContextProviderXai {
                models: xai
                    .models
                    .as_ref()
                    .map(|models| {
                        models
                            .iter()
                            .map(|model| match model {
                                ManifestProviderXaiModel::Name(name) => {
                                    Ok(ResolvedContextProviderXaiModel {
                                        name: name.clone().interpolate(locals)?,
                                        params: None,
                                    })
                                }
                                ManifestProviderXaiModel::Config(config) => {
                                    Ok(ResolvedContextProviderXaiModel {
                                        name: config
                                            .name
                                            .clone()
                                            .interpolate(locals)?,
                                        params: config
                                            .params
                                            .clone()
                                            .map(|params| {
                                                Self::resolve_provider_params(params, locals)
                                            })
                                            .transpose()?,
                                    })
                                }
                            })
                            .collect::<Result<Vec<_>, ManifestError>>()
                    })
                    .transpose()?,
                config: xai
                    .config
                    .as_ref()
                    .map(|config| {
                        Ok::<_, ManifestError>(ResolvedContextProviderXaiConfig {
                            api_key: config
                                .api_key
                                .clone()
                                .interpolate(locals)?,
                        })
                    })
                    .transpose()?,
                params: xai
                    .params
                    .clone()
                    .map(|params| Self::resolve_provider_params(params, locals))
                    .transpose()?,
            }));
        }

        if let Some(gemini) = &self.providers.gemini {
            providers.push(ResolvedContextProvider::Gemini(ResolvedContextProviderGemini {
                models: gemini
                    .models
                    .as_ref()
                    .map(|models| {
                        models
                            .iter()
                            .map(|model| match model {
                                ManifestProviderGeminiModel::Name(name) => {
                                    Ok(ResolvedContextProviderGeminiModel {
                                        name: name.clone().interpolate(locals)?,
                                        params: None,
                                    })
                                }
                                ManifestProviderGeminiModel::Config(config) => {
                                    Ok(ResolvedContextProviderGeminiModel {
                                        name: config
                                            .name
                                            .clone()
                                            .interpolate(locals)?,
                                        params: config
                                            .params
                                            .clone()
                                            .map(|params| {
                                                Self::resolve_provider_params(params, locals)
                                            })
                                            .transpose()?,
                                    })
                                }
                            })
                            .collect::<Result<Vec<_>, ManifestError>>()
                    })
                    .transpose()?,
                config: gemini
                    .config
                    .as_ref()
                    .map(|config| {
                        Ok::<_, ManifestError>(ResolvedContextProviderGeminiConfig {
                            api_key: config
                                .api_key
                                .clone()
                                .interpolate(locals)?,
                        })
                    })
                    .transpose()?,
                params: gemini
                    .params
                    .clone()
                    .map(|params| Self::resolve_provider_params(params, locals))
                    .transpose()?,
            }));
        }

        if let Some(huggingface) = &self.providers.huggingface {
            providers.push(ResolvedContextProvider::HuggingFace(
                ResolvedContextProviderHuggingFace {
                    models: huggingface
                        .models
                        .as_ref()
                        .map(|models| {
                            models
                                .iter()
                                .map(|model| match model {
                                    ManifestProviderHuggingFaceModel::Name(name) => {
                                        Ok(ResolvedContextProviderHuggingFaceModel {
                                            name: name.clone().interpolate(locals)?,
                                            params: None,
                                        })
                                    }
                                    ManifestProviderHuggingFaceModel::Config(config) => {
                                        Ok(ResolvedContextProviderHuggingFaceModel {
                                            name: config
                                                .name
                                                .clone()
                                                .interpolate(locals)?,
                                            params: config
                                                .params
                                                .clone()
                                                .map(|params| {
                                                    Self::resolve_provider_params(params, locals)
                                                })
                                                .transpose()?,
                                        })
                                    }
                                })
                                .collect::<Result<Vec<_>, ManifestError>>()
                        })
                        .transpose()?,
                    config: huggingface
                        .config
                        .as_ref()
                        .map(|config| {
                            Ok::<_, ManifestError>(ResolvedContextProviderHuggingFaceConfig {
                                api_key: config
                                    .api_key
                                    .clone()
                                    .interpolate(locals)?,
                                base_url: config
                                    .base_url
                                    .clone()
                                    .interpolate(locals)?,
                            })
                        })
                        .transpose()?,
                    params: huggingface
                        .params
                        .clone()
                        .map(|params| Self::resolve_provider_params(params, locals))
                        .transpose()?,
                },
            ));
        }

        Ok(providers)
    }

    async fn resolve_agent(&self, locals: &Value) -> Result<ResolvedContextAgent, ManifestError> {
        let agent_label = self.resolve_agent_label()?;
        let agent_block = self
            .agent
            .get(&agent_label)
            .ok_or_else(|| {
                ManifestError::resolution(format!(
                    "agent block with label `{agent_label}` not found in manifest"
                ))
            })?
            .clone();

        Ok(ResolvedContextAgent {
            version: agent_block
                .version
                .interpolate(locals)?,
            description: agent_block
                .description
                .interpolate(locals)?,
            prompt: match agent_block.prompt {
                None => None,
                Some(ManifestAgentPrompt::Prompt(content)) => {
                    Some(ResolvedContextAgentPromptSource::Constant {
                        messages: vec![ResolvedContextAgentPromptMessage {
                            role: ResolvedContextAgentPromptMessageRole::System,
                            content: content.interpolate(locals)?,
                        }],
                    })
                }
                Some(ManifestAgentPrompt::Messages(messages)) => {
                    Some(ResolvedContextAgentPromptSource::Constant {
                        messages: messages
                            .into_iter()
                            .map(|message| {
                                Ok(ResolvedContextAgentPromptMessage {
                                    role: match message.role {
                                        ManifestAgentPromptMessageRole::System => {
                                            ResolvedContextAgentPromptMessageRole::System
                                        }
                                        ManifestAgentPromptMessageRole::User => {
                                            ResolvedContextAgentPromptMessageRole::User
                                        }
                                        ManifestAgentPromptMessageRole::Assistant => {
                                            ResolvedContextAgentPromptMessageRole::Assistant
                                        }
                                    },
                                    content: message.content.interpolate(locals)?,
                                })
                            })
                            .collect::<Result<_, ManifestError>>()?,
                    })
                }
                Some(ManifestAgentPrompt::Source(ManifestAgentPromptSource::Langfuse(prompt))) => {
                    if prompt.label.is_some() && prompt.version.is_some() {
                        return Err(ManifestError::resolution(
                            "Langfuse prompt cannot set both `label` and `version`",
                        ));
                    }

                    Some(ResolvedContextAgentPromptSource::Langfuse(
                        ResolvedContextAgentPromptSourceLangfuse {
                            prompt_name: prompt.prompt_name.interpolate(locals)?,
                            public_key: prompt.public_key.interpolate(locals)?,
                            secret_key: prompt.secret_key.interpolate(locals)?,
                            base_url: prompt.base_url.interpolate(locals)?,
                            label: prompt.label.interpolate(locals)?,
                            version: prompt.version,
                            cache_ttl_seconds: prompt.cache_ttl_seconds,
                            fetch_timeout_seconds: prompt.fetch_timeout_seconds,
                            max_retries: prompt.max_retries,
                        },
                    ))
                }
            },
            capabilities: agent_block
                .capabilities
                .interpolate(locals)?,
            capability_policy: agent_block
                .capability_policy
                .interpolate(locals)?,
            model: ResolvedContextAgentModel {
                provider: agent_block
                    .model
                    .provider
                    .interpolate(locals)?,
                name: agent_block
                    .model
                    .name
                    .interpolate(locals)?,
            },
        })
    }

    async fn resolve_blocks(
        &self,
        loader: &dyn ResourceLoader,
        locals: &Value,
    ) -> Result<HashMap<String, ResolvedContextBlock>, ManifestError> {
        let mut resolved_blocks = HashMap::new();

        for (label, block) in &self.block {
            if label.is_empty() {
                return Err(ManifestError::resolution("block label cannot be empty"));
            }

            let mut generates = HashMap::new();
            for (output_path, template_path) in &block.generates {
                generates.insert(
                    output_path.clone(),
                    ResolvedContextBlockTemplate {
                        path: template_path.clone(),
                        content: loader.load(template_path).await?,
                    },
                );
            }

            let mut contributes = HashMap::new();
            for (ext_point, template_path) in &block.contributes {
                contributes.insert(
                    ext_point.clone(),
                    ResolvedContextBlockTemplate {
                        path: template_path.clone(),
                        content: loader.load(template_path).await?,
                    },
                );
            }

            resolved_blocks.insert(
                label.clone(),
                ResolvedContextBlock {
                    name: label.clone(),
                    description: block
                        .description
                        .clone()
                        .interpolate(locals)?,
                    generates,
                    contributes,
                    dependencies: block.dependencies.clone(),
                },
            );
        }

        Ok(resolved_blocks)
    }

    fn resolve_tools(
        &self,
        assets: &[TransformedAsset],
        locals: &Value,
    ) -> Result<HashMap<String, ResolvedContextTool>, ManifestError> {
        let mut resolved = HashMap::new();

        for (name, tool) in &self.tool {
            if name.is_empty() {
                return Err(ManifestError::resolution("tool name cannot be empty"));
            }

            let kind = match &tool.kind {
                ManifestToolKind::Javascript(js) => {
                    let transformed = assets
                        .iter()
                        .find(|a| matches!(&a.origin, AssetOrigin::Tool { name: tool_name } if tool_name == name))
                        .ok_or_else(|| ManifestError::resolution(
                            format!("no asset found for tool `{name}`.")
                        ))?;

                    ResolvedContextToolKind::Javascript(ResolvedContextToolJavascript {
                        bundle_path: transformed
                            .artifact("source")
                            .ok_or_else(|| ManifestError::resolution(
                                format!("transformed asset for tool `{name}` is missing required `source` artifact.")
                            ))?
                            .as_path()
                            .ok_or_else(|| ManifestError::resolution(
                                format!("artifact `source` for tool `{name}` is not a path artifact.")
                            ))?
                            .to_string_lossy()
                            .to_string(),
                        export_name: js.export.clone().unwrap_or_else(|| name.clone()),
                    })
                }

                ManifestToolKind::Mcp(mcp) => {
                    ResolvedContextToolKind::Mcp(ResolvedContextToolMcp {
                        transport: match mcp {
                            ManifestMcpTool::Stdio { command, args, config } => {
                                ResolvedContextToolMcpTransport::Stdio {
                                    command: command.clone().interpolate(locals)?,
                                    args: args.clone().interpolate(locals)?,
                                    env: config.clone().interpolate(locals)?,
                                }
                            }
                            ManifestMcpTool::Http { url, auth_token, headers } => {
                                ResolvedContextToolMcpTransport::Http {
                                    url: url.clone().interpolate(locals)?,
                                    auth_token: auth_token.clone().interpolate(locals)?,
                                    headers: headers.clone().interpolate(locals)?,
                                }
                            }
                        },
                    })
                }

                ManifestToolKind::A2a(a2a) => {
                    ResolvedContextToolKind::A2a(ResolvedContextToolA2a {
                        url: a2a.url.clone().interpolate(locals)?,
                        auth_token: a2a
                            .auth_token
                            .clone()
                            .interpolate(locals)?,
                        headers: a2a
                            .headers
                            .clone()
                            .interpolate(locals)?,
                        tenant: match &a2a.tenant {
                            ManifestA2aTenant::Inherit => ResolvedContextToolA2aTenant::Inherit,
                            ManifestA2aTenant::None => ResolvedContextToolA2aTenant::None,
                            ManifestA2aTenant::Fixed { id } => {
                                ResolvedContextToolA2aTenant::Fixed {
                                    id: id.clone().interpolate(locals)?,
                                }
                            }
                        },
                        timeout_secs: a2a.timeout_secs.clone(),
                        default_accepted_output_modes: a2a
                            .default_accepted_output_modes
                            .clone()
                            .interpolate(locals)?,
                    })
                }

                ManifestToolKind::Python(py) => {
                    let transformed = assets
                        .iter()
                        .find(|a| matches!(&a.origin, AssetOrigin::Tool { name: tool_name } if tool_name == name))
                        .ok_or_else(|| ManifestError::resolution(
                            format!("no asset found for tool `{name}`.")
                        ))?;

                    let project_path = transformed
                        .artifact("project_path")
                        .ok_or_else(|| ManifestError::resolution(
                            format!("transformed asset for tool `{name}` is missing required `project_path` artifact.")
                        ))?
                        .as_path()
                        .ok_or_else(|| ManifestError::resolution(
                            format!("artifact `project_path` for tool `{name}` is not a path artifact.")
                        ))?
                        .to_string_lossy()
                        .to_string();

                    let site_packages_path = transformed
                        .artifact("site_packages_path")
                        .ok_or_else(|| ManifestError::resolution(
                            format!("transformed asset for tool `{name}` is missing required `site_packages_path` artifact.")
                        ))?
                        .as_path()
                        .ok_or_else(|| ManifestError::resolution(
                            format!("artifact `site_packages_path` for tool `{name}` is not a path artifact.")
                        ))?
                        .to_string_lossy()
                        .to_string();

                    let module_name = transformed
                        .artifact("module_name")
                        .ok_or_else(|| ManifestError::resolution(
                            format!("transformed asset for tool `{name}` is missing required `module_name` artifact.")
                        ))?
                        .as_value()
                        .ok_or_else(|| ManifestError::resolution(
                            format!("artifact `module_name` for tool `{name}` is not a value artifact.")
                        ))?
                        .to_string();

                    ResolvedContextToolKind::Python(ResolvedContextToolPython {
                        project_path,
                        site_packages_path,
                        module_name,
                        export_name: py
                            .export
                            .clone()
                            .unwrap_or_else(|| name.clone()),
                        interpreter: match py.interpreter {
                            ManifestPythonInterpreter::Embedded => {
                                ResolvedContextToolPythonInterpreter::Embedded
                            }
                            ManifestPythonInterpreter::Static => {
                                ResolvedContextToolPythonInterpreter::Static
                            }
                        },
                    })
                }

                ManifestToolKind::Bash(bash) => {
                    ResolvedContextToolKind::Bash(ResolvedContextToolBash {
                        commands: bash
                            .commands
                            .clone()
                            .interpolate(locals)?,
                        cwd: bash.cwd.clone().interpolate(locals)?,
                        env: match &bash.env.kind {
                            ManifestBashEnvKind::Empty => ResolvedContextToolBashEnv::Empty,
                            ManifestBashEnvKind::Inherit => ResolvedContextToolBashEnv::Inherit,
                            ManifestBashEnvKind::Allow => {
                                ResolvedContextToolBashEnv::Allow(bash.env.vars.clone())
                            }
                            ManifestBashEnvKind::Deny => {
                                ResolvedContextToolBashEnv::Deny(bash.env.vars.clone())
                            }
                        },
                        limits: ResolvedContextToolBashLimits {
                            max_execution_time_secs: bash
                                .limits
                                .max_execution_time_secs
                                .unwrap_or(30),
                            max_output_size: bash
                                .limits
                                .max_output_size
                                .unwrap_or(10 * 1024 * 1024),
                            max_command_count: bash
                                .limits
                                .max_command_count
                                .unwrap_or(10_000),
                            max_loop_iterations: bash
                                .limits
                                .max_loop_iterations
                                .unwrap_or(10_000),
                        },
                        shared: bash.shared,
                    })
                }
            };

            resolved.insert(
                name.clone(),
                ResolvedContextTool {
                    name: name.clone(),
                    description: tool
                        .description
                        .clone()
                        .interpolate(locals)?,
                    enabled: tool.enabled.clone(),
                    capabilities: tool
                        .capabilities
                        .clone()
                        .interpolate(locals)?,
                    config: tool
                        .config
                        .clone()
                        .interpolate(locals)?,
                    kind,
                },
            );
        }

        Ok(resolved)
    }

    fn resolve_skills(
        &self,
        assets: &[TransformedAsset],
        locals: &Value,
    ) -> Result<HashMap<String, ResolvedContextSkill>, ManifestError> {
        let mut resolved = HashMap::new();

        for (name, skill) in &self.skill {
            if name.is_empty() {
                return Err(ManifestError::resolution("skill name cannot be empty"));
            }

            let kind = match skill {
                ManifestSkill::Source(_) => {
                    let transformed = assets
                        .iter()
                        .find(|a| matches!(&a.origin, AssetOrigin::Skill { name: skill_name } if skill_name == name))
                        .ok_or_else(|| ManifestError::resolution(
                            format!("no asset found for skill `{name}`.")
                        ))?;

                    let skill_md_artifact = transformed
                        .artifact("skill_md")
                        .ok_or_else(|| ManifestError::resolution(
                            format!("transformed asset for skill `{name}` is missing required `skill_md` artifact.")
                        ))?
                        .as_path()
                        .ok_or_else(|| ManifestError::resolution(
                            format!("artifact `skill_md` for skill `{name}` is not a path artifact.")
                        ))?
                        .clone();

                    ResolvedContextSkillKind::Source(ResolvedContextSkillSource {
                        dir: skill_md_artifact
                            .parent()
                            .ok_or_else(|| {
                                ManifestError::resolution(format!(
                                    "could not determine skill directory for `{name}`."
                                ))
                            })?
                            .to_string_lossy()
                            .to_string(),
                    })
                }

                ManifestSkill::Content(content) => {
                    ResolvedContextSkillKind::Content(ResolvedContextSkillContent {
                        description: content
                            .description
                            .clone()
                            .interpolate(locals)?,
                        content: content
                            .content
                            .clone()
                            .interpolate(locals)?,
                        resources: content
                            .resources
                            .iter()
                            .map(|(path, value)| {
                                Ok((path.clone(), value.clone().interpolate(locals)?))
                            })
                            .collect::<Result<_, ManifestError>>()?,
                    })
                }
            };

            resolved.insert(name.clone(), ResolvedContextSkill { name: name.clone(), kind });
        }

        Ok(resolved)
    }

    fn resolve_http_server(
        &self,
        locals: &Value,
    ) -> Result<Option<ResolvedContextHttpServer>, ManifestError> {
        let Some(http) = &self.http_server else {
            return Ok(None);
        };

        let mut protocols = Vec::new();

        if let Some(ag_ui) = http
            .protocol
            .as_ref()
            .and_then(|protocol| protocol.ag_ui.as_ref())
        {
            protocols.push(ResolvedContextHttpServerProtocol::AgUi(
                ResolvedContextHttpServerProtocolAgUi {
                    path: ag_ui.path.clone().interpolate(locals)?,
                },
            ));
        }

        if let Some(a2a) = http
            .protocol
            .as_ref()
            .and_then(|protocol| protocol.a2a.as_ref())
        {
            protocols.push(ResolvedContextHttpServerProtocol::A2a(
                ResolvedContextHttpServerProtocolA2a {
                    path: a2a.path.clone().interpolate(locals)?,
                },
            ));
        }

        Ok(Some(ResolvedContextHttpServer {
            host: http.host.clone().interpolate(locals)?,
            port: http.port.clone(),
            max_request_size: http.max_request_size.clone(),
            protocols,
        }))
    }

    fn resolve_network(&self, locals: &Value) -> Result<ResolvedContextNetwork, ManifestError> {
        Ok(ResolvedContextNetwork {
            user_agent: self
                .network
                .user_agent
                .clone()
                .interpolate(locals)?,
            headers: self
                .network
                .headers
                .clone()
                .interpolate(locals)?,
            limits: ResolvedContextNetworkLimits {
                connect_timeout_ms: self
                    .network
                    .limits
                    .connect_timeout_ms
                    .clone(),
                read_timeout_ms: self
                    .network
                    .limits
                    .read_timeout_ms
                    .clone(),
                request_timeout_ms: self
                    .network
                    .limits
                    .request_timeout_ms
                    .clone(),
                max_redirects: self
                    .network
                    .limits
                    .max_redirects
                    .clone(),
                max_response_bytes: self
                    .network
                    .limits
                    .max_response_bytes
                    .clone(),
                concurrency_limit: self
                    .network
                    .limits
                    .concurrency_limit
                    .clone(),
            },
            policy: ResolvedContextNetworkPolicy {
                addresses: ResolvedContextNetworkPolicyAddresses {
                    allow_loopback: self
                        .network
                        .policy
                        .addresses
                        .allow_loopback
                        .clone(),
                    allow_private: self
                        .network
                        .policy
                        .addresses
                        .allow_private
                        .clone(),
                    allow_link_local: self
                        .network
                        .policy
                        .addresses
                        .allow_link_local
                        .clone(),
                },
                methods: self
                    .network
                    .policy
                    .methods
                    .clone()
                    .interpolate(locals)?,
                allow: match &self.network.policy.allow {
                    RuntimeValue::Constant(patterns) => RuntimeValue::Constant(
                        patterns
                            .iter()
                            .map(|pattern| pattern.resolve(locals))
                            .collect::<Result<_, ManifestError>>()?,
                    ),
                    RuntimeValue::Runtime { env, default, secret } => RuntimeValue::Runtime {
                        env: env.clone(),
                        default: default
                            .as_ref()
                            .map(|patterns| {
                                patterns
                                    .iter()
                                    .map(|pattern| pattern.resolve(locals))
                                    .collect::<Result<_, ManifestError>>()
                            })
                            .transpose()?,
                        secret: *secret,
                    },
                },
            },
        })
    }

    fn resolve_filesystem(
        &self,
        locals: &Value,
    ) -> Result<ResolvedContextFilesystem, ManifestError> {
        Ok(ResolvedContextFilesystem {
            mounts: self
                .filesystem
                .mounts
                .iter()
                .map(|mount| {
                    Ok(ResolvedContextFilesystemMount {
                        path: mount.path.clone().interpolate(locals)?,
                        backend: mount.backend.resolve(locals)?,
                    })
                })
                .collect::<Result<_, ManifestError>>()?,
        })
    }

    pub async fn resolve(
        self,
        loader: &dyn ResourceLoader,
        assets: &[TransformedAsset],
    ) -> Result<(ResolvedContext, Value), ManifestError> {
        let agent_label = self.resolve_agent_label()?;
        let locals = self.resolve_locals();

        Ok((
            ResolvedContext {
                slug: agent_label
                    .to_lowercase()
                    .replace([' ', '-'], "_"),
                agent_name: agent_label,
                runtime: self.resolve_runtime(&locals)?,
                providers: self.resolve_providers(&locals)?,
                agent: self.resolve_agent(&locals).await?,
                blocks: self
                    .resolve_blocks(loader, &locals)
                    .await?,
                tools: self.resolve_tools(assets, &locals)?,
                skills: self.resolve_skills(assets, &locals)?,
                http_server: self.resolve_http_server(&locals)?,
                network: self.resolve_network(&locals)?,
                filesystem: self.resolve_filesystem(&locals)?,
            },
            self.build.config(),
        ))
    }

    pub fn agent_name(&self) -> Result<String, ManifestError> {
        self.resolve_agent_label()
    }

    pub fn collect_assets(&self) -> Vec<AssetRef> {
        let mut assets = Vec::new();

        for (name, tool) in &self.tool {
            tool.collect_assets(name, &mut assets);
        }

        for (name, skill) in &self.skill {
            skill.collect_assets(name, &mut assets);
        }

        assets
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use async_trait::async_trait;

    use super::*;
    use crate::parser::{
        SpecFormat, SpecParser, errors::ParserError, middleware::hcl::RuntimeFunctionDeserialize,
    };
    use agentc_compiler::{generator::errors::GeneratorError, transformer::types::AssetArtifact};

    struct EmptyLoader;

    #[async_trait]
    impl ResourceLoader for EmptyLoader {
        async fn load(&self, path: &str) -> Result<String, GeneratorError> {
            Err(GeneratorError::resource_not_found(path))
        }
    }

    struct LangfuseManifestFixture;

    impl LangfuseManifestFixture {
        fn manifest(selector: &str) -> Manifest {
            SpecFormat::hcl()
                .with_hcl_deserialize_middleware(RuntimeFunctionDeserialize)
                .deserialize_string::<Manifest>(&format!(
                    r#"
build {{
  archetype = "standalone"
}}

providers {{}}

locals {{
  prompt_folder = "support"
  public_key    = "public"
  secret_key    = "secret"
  base_url      = "https://langfuse.example.com"
  label         = "staging"
}}

agent "assistant" {{
  graph {{
    type = "react"
  }}

  prompt = {{
    source = "langfuse"

    prompt_name           = "${{locals.prompt_folder}}/assistant"
    public_key            = runtime("LANGFUSE_PUBLIC_KEY", "${{locals.public_key}}")
    secret_key            = secret(runtime("LANGFUSE_SECRET_KEY", "${{locals.secret_key}}"))
    base_url              = "${{locals.base_url}}"
    cache_ttl_seconds     = runtime("LANGFUSE_CACHE_TTL", 30)
    fetch_timeout_seconds = runtime("LANGFUSE_FETCH_TIMEOUT", 5)
    max_retries           = runtime("LANGFUSE_MAX_RETRIES", 2)
    {selector}
  }}

  model {{
    provider = "anthropic"
    name     = "claude-haiku-4-5"
  }}
}}
"#
                ))
                .expect("manifest should deserialize")
        }

        async fn resolve(
            selector: &str,
        ) -> Result<ResolvedContextAgentPromptSourceLangfuse, ManifestError> {
            let (resolved, _) = Self::manifest(selector)
                .resolve(&EmptyLoader, &[])
                .await?;
            let Some(ResolvedContextAgentPromptSource::Langfuse(prompt)) = resolved.agent.prompt
            else {
                panic!("prompt should resolve as Langfuse");
            };

            Ok(prompt)
        }
    }

    struct A2aManifestFixture;

    impl A2aManifestFixture {
        fn hcl() -> &'static str {
            r#"
build {
  archetype = "standalone"
}

providers {}

agent "assistant" {
  version = "0.1.0"

  graph {
    type = "react"
  }

  model {
    provider = "anthropic"
    name     = "claude-haiku-4-5"
  }
}

tool "planner" {
  kind          = "a2a"
  description   = "Delegate planning subtasks."
  enabled       = runtime("PLANNER_A2A_ENABLED", true)
  capabilities  = ["a2a:planner"]
  url           = runtime("PLANNER_A2A_URL", "https://planner.example.com")
  auth_token    = secret(runtime("PLANNER_A2A_TOKEN"))

  headers = {
    "X-Agent" = "assistant"
  }

  tenant = {
    policy = "fixed"
    id     = runtime("PLANNER_A2A_TENANT", "tenant-1")
  }

  timeout_secs                  = runtime("PLANNER_A2A_TIMEOUT", 90)
  default_accepted_output_modes = ["text/plain"]
}
"#
        }

        fn manifest() -> Manifest {
            SpecFormat::hcl()
                .with_hcl_deserialize_middleware(RuntimeFunctionDeserialize)
                .deserialize_string::<Manifest>(Self::hcl())
                .expect("manifest should deserialize")
        }
    }

    struct BashManifestFixture;

    impl BashManifestFixture {
        fn manifest(body: &str) -> Manifest {
            SpecFormat::hcl()
                .with_hcl_deserialize_middleware(RuntimeFunctionDeserialize)
                .deserialize_string::<Manifest>(&format!(
                    r#"
build {{
  archetype = "standalone"
}}

providers {{}}

agent "assistant" {{
  graph {{
    type = "react"
  }}

  model {{
    provider = "anthropic"
    name     = "claude-haiku-4-5"
  }}
}}

tool "shell" {{
  kind = "bash"
  {body}
}}
"#
                ))
                .expect("manifest should deserialize")
        }

        async fn resolve(body: &str) -> ResolvedContextToolBash {
            let (context, _) = Self::manifest(body)
                .resolve(&EmptyLoader, &[])
                .await
                .expect("manifest should resolve");
            let ResolvedContextToolKind::Bash(bash) = &context
                .tools
                .get("shell")
                .expect("shell tool should exist")
                .kind
            else {
                panic!("shell tool should resolve as Bash");
            };

            bash.clone()
        }
    }

    #[tokio::test]
    async fn resolves_langfuse_prompt_runtime_configuration() {
        let prompt = LangfuseManifestFixture::resolve("")
            .await
            .expect("Langfuse prompt should resolve");

        assert!(matches!(
            prompt.prompt_name,
            RuntimeValue::Constant(value) if value == "support/assistant"
        ));
        assert!(matches!(
            prompt.public_key,
            RuntimeValue::Runtime {
                env,
                default: Some(default),
                secret: false,
            } if env == "LANGFUSE_PUBLIC_KEY" && default == "public"
        ));
        assert!(matches!(
            prompt.secret_key,
            RuntimeValue::Runtime {
                env,
                default: Some(default),
                secret: true,
            } if env == "LANGFUSE_SECRET_KEY" && default == "secret"
        ));
        assert!(matches!(
            prompt.base_url,
            Some(RuntimeValue::Constant(value))
                if value == "https://langfuse.example.com"
        ));
        assert!(prompt.label.is_none());
        assert!(prompt.version.is_none());
        assert!(matches!(
            prompt.cache_ttl_seconds,
            Some(RuntimeValue::Runtime { env, default: Some(30), .. })
                if env == "LANGFUSE_CACHE_TTL"
        ));
        assert!(matches!(
            prompt.fetch_timeout_seconds,
            Some(RuntimeValue::Runtime { env, default: Some(5), .. })
                if env == "LANGFUSE_FETCH_TIMEOUT"
        ));
        assert!(matches!(
            prompt.max_retries,
            Some(RuntimeValue::Runtime { env, default: Some(2), .. })
                if env == "LANGFUSE_MAX_RETRIES"
        ));
    }

    #[tokio::test]
    async fn resolves_langfuse_label_selector() {
        let prompt = LangfuseManifestFixture::resolve(
            r#"label = runtime("LANGFUSE_LABEL", "${locals.label}")"#,
        )
        .await
        .expect("Langfuse prompt should resolve");

        assert!(matches!(
            prompt.label,
            Some(RuntimeValue::Runtime {
                env,
                default: Some(default),
                secret: false,
            }) if env == "LANGFUSE_LABEL" && default == "staging"
        ));
        assert!(prompt.version.is_none());
    }

    #[tokio::test]
    async fn rejects_conflicting_langfuse_selectors() {
        let error = LangfuseManifestFixture::resolve(
            r#"
label   = "production"
version = 7
"#,
        )
        .await
        .expect_err("conflicting selectors should fail");

        assert_eq!(
            error.to_string(),
            "manifest resolution failed: Langfuse prompt cannot set both `label` and `version`",
        );
    }

    #[test]
    fn manifest_deserializes_a2a_tool() {
        let manifest = A2aManifestFixture::manifest();

        assert!(matches!(
            &manifest
                .tool
                .get("planner")
                .expect("planner tool should exist")
                .kind,
            ManifestToolKind::A2a(_)
        ));
    }

    #[tokio::test]
    async fn manifest_resolves_a2a_tool() {
        let (resolved, _) = A2aManifestFixture::manifest()
            .resolve(&EmptyLoader, &[])
            .await
            .expect("manifest should resolve");

        let tool = resolved
            .tools
            .get("planner")
            .expect("planner tool should resolve");

        assert_eq!(tool.description.as_deref(), Some("Delegate planning subtasks."));
        assert!(matches!(
            &tool.enabled,
            RuntimeValue::Runtime { env, default, .. }
                if env == "PLANNER_A2A_ENABLED" && default == &Some(true)
        ));
        assert_eq!(tool.capabilities, vec!["a2a:planner"]);

        let ResolvedContextToolKind::A2a(a2a) = &tool.kind else {
            panic!("planner should resolve as A2A");
        };

        assert!(matches!(
            &a2a.url,
            RuntimeValue::Runtime { env, default, .. }
                if env == "PLANNER_A2A_URL"
                    && default.as_deref() == Some("https://planner.example.com")
        ));
        assert!(matches!(
            &a2a.auth_token,
            Some(RuntimeValue::Runtime { env, secret, .. })
                if env == "PLANNER_A2A_TOKEN" && *secret
        ));
        assert!(matches!(
            a2a.headers.get("X-Agent"),
            Some(RuntimeValue::Constant(value)) if value == "assistant"
        ));
        assert!(matches!(
            &a2a.tenant,
            ResolvedContextToolA2aTenant::Fixed {
                id: RuntimeValue::Runtime { env, default, .. },
            } if env == "PLANNER_A2A_TENANT"
                && default.as_deref() == Some("tenant-1")
        ));
        assert!(matches!(
            &a2a.timeout_secs,
            Some(RuntimeValue::Runtime { env, default, .. })
                if env == "PLANNER_A2A_TIMEOUT" && default == &Some(90)
        ));
        assert_eq!(a2a.default_accepted_output_modes, vec!["text/plain"]);
    }

    #[test]
    fn a2a_tool_collects_no_assets() {
        assert!(
            A2aManifestFixture::manifest()
                .collect_assets()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn spec_parser_preserves_nested_network_port_conversion() {
        let manifest = SpecParser::<Manifest>::default()
            .with_content(
                r#"
build {
  archetype = "standalone"
}

providers {}

agent "assistant" {
  graph {
    type = "react"
  }

  model {
    provider = "anthropic"
    name     = "claude-haiku-4-5"
  }
}

network {
  policy {
    allow = [{ protocol = "http", port = 8086 }]
  }
}
"#,
                SpecFormat::hcl().with_hcl_deserialize_middleware(RuntimeFunctionDeserialize),
            )
            .parse()
            .await
            .expect("manifest should parse");

        assert!(matches!(
            &manifest.network.policy.allow,
            RuntimeValue::Constant(patterns)
                if patterns.len() == 1
                    && patterns[0].protocol.as_deref() == Some("http")
                    && patterns[0].port.as_deref() == Some("8086")
        ));
    }

    #[tokio::test]
    async fn network_allow_type_error_names_the_nested_field() {
        let error = SpecParser::<Manifest>::default()
            .with_content(
                r#"
build {
  archetype = "standalone"
}

providers {}

agent "assistant" {
  graph {
    type = "react"
  }

  model {
    provider = "anthropic"
    name     = "claude-haiku-4-5"
  }
}

network {
  policy {
    allow = [{ protocol = "http", port = [1] }]
  }
}
"#,
                SpecFormat::hcl().with_hcl_deserialize_middleware(RuntimeFunctionDeserialize),
            )
            .parse()
            .await
            .unwrap_err();

        assert!(
            matches!(
                &error,
                ParserError::Parser(config::ConfigError::Type {
                    key: Some(path),
                    ..
                }) if path == "network.policy.allow[0]port"
            ),
            "{error}"
        );
        assert!(
            error
                .to_string()
                .contains("expected a string")
        );
    }

    #[tokio::test]
    async fn spec_parser_preserves_typed_runtime_default() {
        let manifest = SpecParser::<Manifest>::default()
            .with_content(
                r#"
build {
  archetype = "standalone"
}

providers {}

agent "assistant" {
  graph {
    type = "react"
  }

  model {
    provider = "anthropic"
    name     = "claude-haiku-4-5"
  }
}

http_server {
  port = runtime("HTTP_PORT", "8080")
}
"#,
                SpecFormat::hcl().with_hcl_deserialize_middleware(RuntimeFunctionDeserialize),
            )
            .parse()
            .await
            .expect("manifest should parse");

        assert!(matches!(
            &manifest.http_server.expect("HTTP server should be present").port,
            RuntimeValue::Runtime {
                env,
                default: Some(8080),
                secret: false,
            } if env == "HTTP_PORT"
        ));
    }

    #[tokio::test]
    async fn network_manifest_block_resolves_constants_and_runtime_leaves() {
        let manifest = SpecFormat::hcl()
            .with_hcl_deserialize_middleware(RuntimeFunctionDeserialize)
            .deserialize_string::<Manifest>(
                r#"
build {
  archetype = "standalone"
}

providers {}

agent "assistant" {
  graph {
    type = "react"
  }

  model {
    provider = "anthropic"
    name     = "claude-haiku-4-5"
  }
}

network {
  limits {
    max_redirects = 3
  }

  policy {
    addresses {
      allow_private = runtime("NETWORK_ALLOW_PRIVATE", false)
    }

    allow = [{ hostname = "api.internal.example.com" }]
  }
}
"#,
            )
            .expect("manifest should deserialize");

        let (resolved, _) = manifest
            .resolve(&EmptyLoader, &[])
            .await
            .expect("manifest should resolve");

        assert_eq!(resolved.network.limits.max_redirects, RuntimeValue::Constant(3));
        assert!(matches!(
            &resolved.network.policy.addresses.allow_private,
            RuntimeValue::Runtime { env, default, .. }
                if env == "NETWORK_ALLOW_PRIVATE" && default == &Some(false)
        ));
        assert!(matches!(
            &resolved.network.policy.allow,
            RuntimeValue::Constant(patterns)
                if patterns.len() == 1
                    && patterns[0].hostname.as_deref() == Some("api.internal.example.com")
        ));
        assert!(matches!(
            &resolved.network.user_agent,
            RuntimeValue::Runtime { env, default: Some(None), .. } if env == "NETWORK_USER_AGENT"
        ));
    }

    #[tokio::test]
    async fn filesystem_manifest_block_resolves_nested_backends_and_defaults_to_memory() {
        let default_manifest = SpecFormat::hcl()
            .with_hcl_deserialize_middleware(RuntimeFunctionDeserialize)
            .deserialize_string::<Manifest>(
                r#"
build {
  archetype = "standalone"
}

providers {}

agent "assistant" {
  graph {
    type = "react"
  }

  model {
    provider = "anthropic"
    name     = "claude-haiku-4-5"
  }
}
"#,
            )
            .expect("manifest should deserialize");

        let (resolved, _) = default_manifest
            .resolve(&EmptyLoader, &[])
            .await
            .expect("manifest should resolve");

        assert_eq!(resolved.filesystem.mounts.len(), 1);
        assert_eq!(resolved.filesystem.mounts[0].path, "/");
        assert!(matches!(
            resolved.filesystem.mounts[0].backend,
            ResolvedContextFilesystemBackend::Memory
        ));

        let overlay_manifest = SpecFormat::hcl()
            .with_hcl_deserialize_middleware(RuntimeFunctionDeserialize)
            .deserialize_string::<Manifest>(
                r#"
build {
  archetype = "standalone"
}

providers {}

agent "assistant" {
  graph {
    type = "react"
  }

  model {
    provider = "anthropic"
    name     = "claude-haiku-4-5"
  }
}

filesystem {
  mounts = [{
    path = "/workspace"
    backend = {
      kind = "overlay"
      upper = { kind = "memory" }
      lower = { kind = "host", root = "/var/lib/agent/base" }
    }
  }]
}
"#,
            )
            .expect("manifest should deserialize");

        let (resolved, _) = overlay_manifest
            .resolve(&EmptyLoader, &[])
            .await
            .expect("manifest should resolve");

        assert_eq!(resolved.filesystem.mounts.len(), 1);
        assert_eq!(resolved.filesystem.mounts[0].path, "/workspace");
        assert!(matches!(
            &resolved.filesystem.mounts[0].backend,
            ResolvedContextFilesystemBackend::Overlay { upper, lower }
                if matches!(**upper, ResolvedContextFilesystemBackend::Memory)
                    && matches!(
                        **lower,
                        ResolvedContextFilesystemBackend::Host { ref root, .. }
                            if root == "/var/lib/agent/base"
                    )
        ));
    }

    #[tokio::test]
    async fn resolves_bash_tool_configuration() {
        let bash = BashManifestFixture::resolve(
            r#"
commands = ["git", "rg"]
cwd      = "/workspace"
shared   = true

env {
  kind = "allow"
  vars = ["HOME", "PATH"]
}

limits {
  max_execution_time_secs = 7
  max_output_size          = 512
  max_command_count        = 23
  max_loop_iterations      = 29
}
"#,
        )
        .await;

        assert_eq!(bash.commands, ["git", "rg"]);
        assert_eq!(bash.cwd, "/workspace");
        assert!(matches!(
            bash.env,
            ResolvedContextToolBashEnv::Allow(vars)
                if vars == ["HOME", "PATH"]
        ));
        assert_eq!(bash.limits.max_execution_time_secs, 7);
        assert_eq!(bash.limits.max_output_size, 512);
        assert_eq!(bash.limits.max_command_count, 23);
        assert_eq!(bash.limits.max_loop_iterations, 29);
        assert!(bash.shared);
    }

    #[tokio::test]
    async fn resolves_bash_tool_defaults() {
        let bash = BashManifestFixture::resolve("").await;

        assert!(bash.commands.is_empty());
        assert_eq!(bash.cwd, "/home/agent");
        assert!(matches!(bash.env, ResolvedContextToolBashEnv::Empty));
        assert_eq!(bash.limits.max_execution_time_secs, 30);
        assert_eq!(bash.limits.max_output_size, 10 * 1024 * 1024);
        assert_eq!(bash.limits.max_command_count, 10_000);
        assert_eq!(bash.limits.max_loop_iterations, 10_000);
        assert!(!bash.shared);
    }

    #[tokio::test]
    async fn locals_interpolate_resolved_configuration_values() {
        let mut manifest = A2aManifestFixture::manifest();
        manifest
            .locals
            .insert("value".to_owned(), RuntimeValue::constant("interpolated".to_owned()));
        manifest.runtime.default_tenant_id =
            RuntimeValue::default_runtime("TENANT", "${locals.value}".to_owned());
        manifest.providers.anthropic = Some(ManifestProviderAnthropic {
            models: Some(vec![
                ManifestProviderAnthropicModel::Name("${locals.value}".to_owned()),
                ManifestProviderAnthropicModel::Config(ManifestProviderAnthropicModelConfig {
                    name: "${locals.value}".to_owned(),
                    params: Some(ManifestProviderParams {
                        max_tokens: None,
                        temperature: None,
                        top_p: None,
                        top_k: None,
                        stop_sequences: Some(RuntimeValue::constant(vec![
                            "${locals.value}".to_owned(),
                        ])),
                        frequency_penalty: None,
                        presence_penalty: None,
                        seed: None,
                        provider_params: None,
                    }),
                }),
            ]),
            config: Some(ManifestProviderAnthropicConfig {
                api_key: Some(RuntimeValue::default_runtime(
                    "ANTHROPIC_API_KEY",
                    "${locals.value}".to_owned(),
                )),
                base_url: Some(RuntimeValue::constant("${locals.value}".to_owned())),
            }),
            params: Some(ManifestProviderParams {
                max_tokens: None,
                temperature: None,
                top_p: None,
                top_k: None,
                stop_sequences: None,
                frequency_penalty: None,
                presence_penalty: None,
                seed: None,
                provider_params: Some(RuntimeValue::constant(
                    json!({ "nested": ["${locals.value}"] }),
                )),
            }),
        });

        let agent = manifest
            .agent
            .get_mut("assistant")
            .unwrap();
        agent.version = "${locals.value}".to_owned();
        agent.description = Some("${locals.value}".to_owned());
        agent.prompt = Some(ManifestAgentPrompt::Prompt("${locals.value}".to_owned()));
        agent.model.name = RuntimeValue::constant("${locals.value}".to_owned());

        manifest.block.insert(
            "${locals.value}".to_owned(),
            ManifestBlock {
                description: Some("${locals.value}".to_owned()),
                generates: HashMap::new(),
                contributes: HashMap::new(),
                dependencies: HashMap::from([("dep".to_owned(), "1".to_owned())]),
            },
        );

        let tool = manifest
            .tool
            .get_mut("planner")
            .unwrap();
        tool.description = Some("${locals.value}".to_owned());
        tool.capabilities = vec!["${locals.value}".to_owned()];
        tool.config = HashMap::from([(
            "${locals.value}".to_owned(),
            RuntimeValue::default_runtime("TOOL_CONFIG", "${locals.value}".to_owned()),
        )]);
        let ManifestToolKind::A2a(a2a) = &mut tool.kind else {
            panic!("planner should be an A2A tool");
        };
        a2a.url = RuntimeValue::default_runtime(
            "PLANNER_A2A_URL",
            "https://${locals.value}.example.com".to_owned(),
        );
        a2a.headers = HashMap::from([(
            "${locals.value}".to_owned(),
            RuntimeValue::constant("${locals.value}".to_owned()),
        )]);
        a2a.tenant = ManifestA2aTenant::Fixed {
            id: RuntimeValue::constant("${locals.value}".to_owned()),
        };
        a2a.default_accepted_output_modes = vec!["${locals.value}".to_owned()];

        manifest.skill.insert(
            "inline".to_owned(),
            ManifestSkill::Content(ManifestSkillContent {
                description: "${locals.value}".to_owned(),
                content: "${locals.value}".to_owned(),
                resources: HashMap::from([(
                    "${locals.value}.txt".to_owned(),
                    "${locals.value}".to_owned(),
                )]),
            }),
        );
        manifest.http_server = Some(ManifestHttpServer {
            host: RuntimeValue::default_runtime("HTTP_HOST", "${locals.value}".to_owned()),
            port: RuntimeValue::constant(8080),
            max_request_size: RuntimeValue::constant(1024),
            protocol: Some(ManifestHttpServerProtocol {
                ag_ui: Some(ManifestHttpServerProtocolAgUi { path: "/${locals.value}".to_owned() }),
                a2a: Some(ManifestHttpServerProtocolA2a { path: "/${locals.value}".to_owned() }),
            }),
        });
        manifest.network.user_agent =
            RuntimeValue::default_runtime("USER_AGENT", Some("${locals.value}".to_owned()));
        manifest.network.headers = RuntimeValue::default_runtime(
            "HEADERS",
            BTreeMap::from([("${locals.value}".to_owned(), "${locals.value}".to_owned())]),
        );
        manifest.network.policy.methods = RuntimeValue::default_runtime(
            "METHODS",
            Some(BTreeSet::from(["${locals.value}".to_owned()])),
        );
        manifest.network.policy.allow = RuntimeValue::default_runtime(
            "PATTERNS",
            vec![ManifestNetworkUrlPattern {
                protocol: Some("${locals.value}".to_owned()),
                hostname: Some("${locals.value}".to_owned()),
                port: Some("${locals.value}".to_owned()),
                pathname: Some("${locals.value}".to_owned()),
            }],
        );
        manifest.filesystem.mounts = vec![ManifestFilesystemMount {
            path: "/${locals.value}".to_owned(),
            backend: ManifestFilesystemBackend::Overlay {
                upper: Box::new(ManifestFilesystemBackend::Memory),
                lower: Box::new(ManifestFilesystemBackend::Host {
                    root: "/${locals.value}".to_owned(),
                    follow_symlinks: false,
                }),
            },
        }];

        let (resolved, _) = manifest
            .resolve(&EmptyLoader, &[])
            .await
            .unwrap();

        assert_eq!(
            resolved
                .runtime
                .default_tenant_id
                .as_runtime(),
            Some(("TENANT", Some(&"interpolated".to_owned()), false)),
        );
        let ResolvedContextProvider::Anthropic(provider) = &resolved.providers[0] else {
            panic!("provider should be Anthropic");
        };
        let models = provider.models.as_ref().unwrap();
        assert_eq!(models[0].name, "interpolated");
        assert_eq!(models[1].name, "interpolated");
        assert_eq!(
            models[1]
                .params
                .as_ref()
                .unwrap()
                .stop_sequences
                .as_ref()
                .unwrap()
                .default_value()
                .unwrap(),
            &vec!["interpolated".to_owned()],
        );
        assert_eq!(
            provider
                .config
                .as_ref()
                .unwrap()
                .api_key
                .as_ref()
                .unwrap()
                .default_value()
                .map(String::as_str),
            Some("interpolated"),
        );
        assert_eq!(
            provider
                .params
                .as_ref()
                .unwrap()
                .provider_params
                .as_ref()
                .unwrap()
                .default_value(),
            Some(&json!({ "nested": ["interpolated"] })),
        );
        assert_eq!(resolved.agent.version, "interpolated");
        assert_eq!(resolved.agent.description.as_deref(), Some("interpolated"));
        let Some(ResolvedContextAgentPromptSource::Constant { messages }) = &resolved.agent.prompt
        else {
            panic!("agent prompt should be constant");
        };
        assert_eq!(messages[0].content, "interpolated");
        assert_eq!(
            resolved
                .agent
                .model
                .name
                .default_value()
                .map(String::as_str),
            Some("interpolated"),
        );
        let block = resolved
            .blocks
            .get("${locals.value}")
            .unwrap();
        assert_eq!(block.name, "${locals.value}");
        assert_eq!(block.description.as_deref(), Some("interpolated"));
        assert_eq!(
            block
                .dependencies
                .get("dep")
                .map(String::as_str),
            Some("1")
        );

        let tool = resolved.tools.get("planner").unwrap();
        assert_eq!(tool.description.as_deref(), Some("interpolated"));
        assert_eq!(tool.capabilities, ["interpolated"]);
        assert_eq!(
            tool.config
                .get("interpolated")
                .unwrap()
                .default_value()
                .map(String::as_str),
            Some("interpolated"),
        );
        let ResolvedContextToolKind::A2a(a2a) = &tool.kind else {
            panic!("planner should resolve as A2A");
        };
        assert_eq!(
            a2a.url
                .default_value()
                .map(String::as_str),
            Some("https://interpolated.example.com"),
        );
        assert_eq!(
            a2a.headers
                .get("interpolated")
                .unwrap()
                .default_value()
                .map(String::as_str),
            Some("interpolated"),
        );
        assert!(matches!(
            &a2a.tenant,
            ResolvedContextToolA2aTenant::Fixed { id }
                if id.default_value().map(String::as_str) == Some("interpolated")
        ));
        assert_eq!(a2a.default_accepted_output_modes, ["interpolated"]);

        let ResolvedContextSkillKind::Content(skill) = &resolved
            .skills
            .get("inline")
            .unwrap()
            .kind
        else {
            panic!("inline skill should have content");
        };
        assert_eq!(skill.description, "interpolated");
        assert_eq!(skill.content, "interpolated");
        assert_eq!(
            skill
                .resources
                .get("${locals.value}.txt")
                .map(String::as_str),
            Some("interpolated"),
        );

        let http = resolved.http_server.as_ref().unwrap();
        assert_eq!(
            http.host
                .default_value()
                .map(String::as_str),
            Some("interpolated")
        );
        assert!(matches!(
            &http.protocols[0],
            ResolvedContextHttpServerProtocol::AgUi(ag_ui)
                if ag_ui.path == "/interpolated"
        ));
        assert!(matches!(
            &http.protocols[1],
            ResolvedContextHttpServerProtocol::A2a(a2a)
                if a2a.path == "/interpolated"
        ));
        assert_eq!(
            resolved
                .network
                .user_agent
                .default_value(),
            Some(&Some("interpolated".to_owned())),
        );
        assert_eq!(
            resolved
                .network
                .headers
                .default_value()
                .unwrap()
                .get("interpolated"),
            Some(&"interpolated".to_owned()),
        );
        assert!(
            resolved
                .network
                .policy
                .methods
                .default_value()
                .unwrap()
                .as_ref()
                .unwrap()
                .contains("interpolated")
        );
        let pattern = &resolved
            .network
            .policy
            .allow
            .default_value()
            .unwrap()[0];
        assert_eq!(pattern.protocol.as_deref(), Some("interpolated"));
        assert_eq!(pattern.hostname.as_deref(), Some("interpolated"));
        assert_eq!(pattern.port.as_deref(), Some("interpolated"));
        assert_eq!(pattern.pathname.as_deref(), Some("interpolated"));

        let mount = &resolved.filesystem.mounts[0];
        assert_eq!(mount.path, "/interpolated");
        assert!(matches!(
            &mount.backend,
            ResolvedContextFilesystemBackend::Overlay { lower, .. }
                if matches!(
                    lower.as_ref(),
                    ResolvedContextFilesystemBackend::Host { root, follow_symlinks }
                        if root == "/interpolated" && !follow_symlinks
                )
        ));
    }

    #[tokio::test]
    async fn missing_local_in_tool_url_fails_resolution() {
        let mut manifest = A2aManifestFixture::manifest();
        let tool = manifest
            .tool
            .get_mut("planner")
            .unwrap();
        let ManifestToolKind::A2a(a2a) = &mut tool.kind else {
            panic!("planner should be an A2A tool");
        };
        a2a.url = RuntimeValue::constant("https://${locals.missing}".to_owned());

        let error = manifest
            .resolve(&EmptyLoader, &[])
            .await
            .unwrap_err();

        assert!(matches!(
            error,
            ManifestError::Resolution(message)
                if message.contains("${locals.missing}")
        ));
    }

    #[tokio::test]
    async fn asset_sources_exports_and_resource_paths_keep_their_identity() {
        let mut manifest = A2aManifestFixture::manifest();
        manifest
            .locals
            .insert("value".to_owned(), RuntimeValue::constant("interpolated".to_owned()));
        manifest.tool.insert(
            "js_${locals.value}".to_owned(),
            ManifestTool {
                description: None,
                enabled: RuntimeValue::constant(true),
                capabilities: Vec::new(),
                config: HashMap::new(),
                kind: ManifestToolKind::Javascript(ManifestJavascriptTool {
                    source: "${locals.value}/tool".to_owned(),
                    export: Some("${locals.value}".to_owned()),
                }),
            },
        );
        manifest.skill.insert(
            "skill_${locals.value}".to_owned(),
            ManifestSkill::Source(ManifestSkillSource {
                source: "${locals.value}/skill".to_owned(),
            }),
        );

        let references = manifest.collect_assets();
        assert!(references.iter().any(|asset| {
            asset.uri == "${locals.value}/tool"
                && matches!(&asset.origin, AssetOrigin::Tool { name } if name == "js_${locals.value}")
        }));
        assert!(references.iter().any(|asset| {
            asset.uri == "${locals.value}/skill"
                && matches!(&asset.origin, AssetOrigin::Skill { name } if name == "skill_${locals.value}")
        }));

        let assets = [
            TransformedAsset {
                uri: "${locals.value}/tool".to_owned(),
                origin: AssetOrigin::tool("js_${locals.value}"),
                artifacts: vec![AssetArtifact::path("source", "/tmp/agentc-tool.js")],
            },
            TransformedAsset {
                uri: "${locals.value}/skill".to_owned(),
                origin: AssetOrigin::skill("skill_${locals.value}"),
                artifacts: vec![AssetArtifact::path(
                    "skill_md",
                    "/tmp/agentc-skill/SKILL.md",
                )],
            },
        ];
        let (resolved, _) = manifest
            .resolve(&EmptyLoader, &assets)
            .await
            .unwrap();
        let ResolvedContextToolKind::Javascript(tool) = &resolved
            .tools
            .get("js_${locals.value}")
            .unwrap()
            .kind
        else {
            panic!("JavaScript tool should resolve");
        };
        assert_eq!(tool.export_name, "${locals.value}");
        let ResolvedContextSkillKind::Source(skill) = &resolved
            .skills
            .get("skill_${locals.value}")
            .unwrap()
            .kind
        else {
            panic!("source skill should resolve");
        };
        assert_eq!(skill.dir, "/tmp/agentc-skill");
    }
}
