// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

pub mod agent;
pub mod block;
pub mod build;
pub mod errors;
pub mod graph;
pub mod http_server;
pub mod interpolate;
pub mod observability;
pub mod provider;
pub mod runtime;
pub mod skill;
pub mod tool;

pub use agent::*;
pub use block::*;
pub use build::*;
pub use graph::*;
pub use http_server::*;
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

    fn resolve_runtime(&self) -> ResolvedContextRuntime {
        ResolvedContextRuntime {
            default_tenant_id: self.runtime.default_tenant_id.clone(),
        }
    }

    fn resolve_provider_params(p: ManifestProviderParams) -> ResolvedContextProviderParams {
        ResolvedContextProviderParams {
            max_tokens: p.max_tokens,
            temperature: p.temperature,
            top_p: p.top_p,
            top_k: p.top_k,
            stop_sequences: p.stop_sequences,
            frequency_penalty: p.frequency_penalty,
            presence_penalty: p.presence_penalty,
            seed: p.seed,
            provider_params: p.provider_params,
        }
    }

    fn resolve_providers(&self) -> Vec<ResolvedContextProvider> {
        let mut providers = Vec::new();

        if let Some(anthropic) = &self.providers.anthropic {
            providers.push(ResolvedContextProvider::Anthropic(ResolvedContextProviderAnthropic {
                models: anthropic.models.as_ref().map(|models| {
                    models
                        .iter()
                        .map(|m| match m {
                            ManifestProviderAnthropicModel::Name(name) => {
                                ResolvedContextProviderAnthropicModel {
                                    name: name.clone(),
                                    params: None,
                                }
                            }
                            ManifestProviderAnthropicModel::Config(c) => {
                                ResolvedContextProviderAnthropicModel {
                                    name: c.name.clone(),
                                    params: c
                                        .params
                                        .clone()
                                        .map(Self::resolve_provider_params),
                                }
                            }
                        })
                        .collect()
                }),
                config: anthropic
                    .config
                    .as_ref()
                    .map(|c| ResolvedContextProviderAnthropicConfig {
                        api_key: c.api_key.clone(),
                        base_url: c.base_url.clone(),
                    }),
                params: anthropic
                    .params
                    .clone()
                    .map(Self::resolve_provider_params),
            }));
        }

        if let Some(openai) = &self.providers.openai {
            providers.push(ResolvedContextProvider::OpenAi(ResolvedContextProviderOpenAi {
                models: openai.models.as_ref().map(|models| {
                    models
                        .iter()
                        .map(|m| match m {
                            ManifestProviderOpenAiModel::Name(name) => {
                                ResolvedContextProviderOpenAiModel {
                                    name: name.clone(),
                                    params: None,
                                }
                            }
                            ManifestProviderOpenAiModel::Config(c) => {
                                ResolvedContextProviderOpenAiModel {
                                    name: c.name.clone(),
                                    params: c
                                        .params
                                        .clone()
                                        .map(Self::resolve_provider_params),
                                }
                            }
                        })
                        .collect()
                }),
                config: openai
                    .config
                    .as_ref()
                    .map(|c| ResolvedContextProviderOpenAiConfig {
                        api_key: c.api_key.clone(),
                        base_url: c.base_url.clone(),
                    }),
                params: openai
                    .params
                    .clone()
                    .map(Self::resolve_provider_params),
            }));
        }

        if let Some(ollama) = &self.providers.ollama {
            providers.push(ResolvedContextProvider::Ollama(ResolvedContextProviderOllama {
                models: ollama.models.as_ref().map(|models| {
                    models
                        .iter()
                        .map(|m| match m {
                            ManifestProviderOllamaModel::Name(name) => {
                                ResolvedContextProviderOllamaModel {
                                    name: name.clone(),
                                    params: None,
                                }
                            }
                            ManifestProviderOllamaModel::Config(c) => {
                                ResolvedContextProviderOllamaModel {
                                    name: c.name.clone(),
                                    params: c
                                        .params
                                        .clone()
                                        .map(Self::resolve_provider_params),
                                }
                            }
                        })
                        .collect()
                }),
                config: ollama
                    .config
                    .as_ref()
                    .map(|c| ResolvedContextProviderOllamaConfig { base_url: c.base_url.clone() }),
                params: ollama
                    .params
                    .clone()
                    .map(Self::resolve_provider_params),
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
                                .map(|m| match m {
                                    ManifestProviderOpenRouterModel::Name(name) => {
                                        ResolvedContextProviderOpenRouterModel {
                                            name: name.clone(),
                                            params: None,
                                        }
                                    }
                                    ManifestProviderOpenRouterModel::Config(c) => {
                                        ResolvedContextProviderOpenRouterModel {
                                            name: c.name.clone(),
                                            params: c
                                                .params
                                                .clone()
                                                .map(Self::resolve_provider_params),
                                        }
                                    }
                                })
                                .collect()
                        }),
                    config: openrouter.config.as_ref().map(|c| {
                        ResolvedContextProviderOpenRouterConfig { api_key: c.api_key.clone() }
                    }),
                    params: openrouter
                        .params
                        .clone()
                        .map(Self::resolve_provider_params),
                },
            ));
        }

        if let Some(xai) = &self.providers.xai {
            providers.push(ResolvedContextProvider::Xai(ResolvedContextProviderXai {
                models: xai.models.as_ref().map(|models| {
                    models
                        .iter()
                        .map(|m| match m {
                            ManifestProviderXaiModel::Name(name) => {
                                ResolvedContextProviderXaiModel { name: name.clone(), params: None }
                            }
                            ManifestProviderXaiModel::Config(c) => {
                                ResolvedContextProviderXaiModel {
                                    name: c.name.clone(),
                                    params: c
                                        .params
                                        .clone()
                                        .map(Self::resolve_provider_params),
                                }
                            }
                        })
                        .collect()
                }),
                config: xai
                    .config
                    .as_ref()
                    .map(|c| ResolvedContextProviderXaiConfig { api_key: c.api_key.clone() }),
                params: xai
                    .params
                    .clone()
                    .map(Self::resolve_provider_params),
            }));
        }

        if let Some(gemini) = &self.providers.gemini {
            providers.push(ResolvedContextProvider::Gemini(ResolvedContextProviderGemini {
                models: gemini.models.as_ref().map(|models| {
                    models
                        .iter()
                        .map(|m| match m {
                            ManifestProviderGeminiModel::Name(name) => {
                                ResolvedContextProviderGeminiModel {
                                    name: name.clone(),
                                    params: None,
                                }
                            }
                            ManifestProviderGeminiModel::Config(c) => {
                                ResolvedContextProviderGeminiModel {
                                    name: c.name.clone(),
                                    params: c
                                        .params
                                        .clone()
                                        .map(Self::resolve_provider_params),
                                }
                            }
                        })
                        .collect()
                }),
                config: gemini
                    .config
                    .as_ref()
                    .map(|c| ResolvedContextProviderGeminiConfig { api_key: c.api_key.clone() }),
                params: gemini
                    .params
                    .clone()
                    .map(Self::resolve_provider_params),
            }));
        }

        if let Some(huggingface) = &self.providers.huggingface {
            providers.push(ResolvedContextProvider::HuggingFace(
                ResolvedContextProviderHuggingFace {
                    models: huggingface.models.as_ref().map(|models| {
                        models
                            .iter()
                            .map(|m| match m {
                                ManifestProviderHuggingFaceModel::Name(name) => {
                                    ResolvedContextProviderHuggingFaceModel {
                                        name: name.clone(),
                                        params: None,
                                    }
                                }
                                ManifestProviderHuggingFaceModel::Config(c) => {
                                    ResolvedContextProviderHuggingFaceModel {
                                        name: c.name.clone(),
                                        params: c
                                            .params
                                            .clone()
                                            .map(Self::resolve_provider_params),
                                    }
                                }
                            })
                            .collect()
                    }),
                    config: huggingface.config.as_ref().map(|c| {
                        ResolvedContextProviderHuggingFaceConfig {
                            api_key: c.api_key.clone(),
                            base_url: c.base_url.clone(),
                        }
                    }),
                    params: huggingface
                        .params
                        .clone()
                        .map(Self::resolve_provider_params),
                },
            ));
        }

        providers
    }

    async fn resolve_agent(&self) -> Result<ResolvedContextAgent, ManifestError> {
        let agent_label = self.resolve_agent_label()?;
        let locals = self.resolve_locals();
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
            version: agent_block.version.interpolate(&locals),
            description: agent_block
                .description
                .map(|d| d.interpolate(&locals)),
            prompt: agent_block
                .prompt
                .map(|prompt| match prompt {
                    ManifestAgentPrompt::Prompt(content) => {
                        vec![ResolvedContextAgentPromptMessage {
                            role: ResolvedContextAgentPromptMessageRole::System,
                            content: content.interpolate(&locals),
                        }]
                    }
                    ManifestAgentPrompt::Messages(messages) => messages
                        .into_iter()
                        .map(|message| ResolvedContextAgentPromptMessage {
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
                            content: message.content.interpolate(&locals),
                        })
                        .collect(),
                }),
            capabilities: agent_block
                .capabilities
                .clone()
                .map(|cv| cv.interpolate(&locals)),
            capability_policy: agent_block
                .capability_policy
                .clone()
                .map(|cp| cp.interpolate(&locals)),
            model: ResolvedContextAgentModel {
                provider: agent_block
                    .model
                    .provider
                    .interpolate(&locals),
                name: agent_block
                    .model
                    .name
                    .interpolate(&locals),
            },
        })
    }

    async fn resolve_blocks(
        &self,
        loader: &dyn ResourceLoader,
    ) -> Result<HashMap<String, ResolvedContextBlock>, ManifestError> {
        let locals = self.resolve_locals();
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
                        .as_ref()
                        .map(|d| d.clone().interpolate(&locals)),
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

                ManifestToolKind::Mcp(mcp) => ResolvedContextToolKind::Mcp(
                    ResolvedContextToolMcp {
                        transport: match mcp {
                            ManifestMcpTool::Stdio { command, args, config } => {
                                ResolvedContextToolMcpTransport::Stdio {
                                    command: command.clone(),
                                    args: args.clone(),
                                    env: config.clone(),
                                }
                            }
                            ManifestMcpTool::Http { url, auth_token, headers } => {
                                ResolvedContextToolMcpTransport::Http {
                                    url: url.clone(),
                                    auth_token: auth_token.clone(),
                                    headers: headers.clone(),
                                }
                            }
                        },
                    }
                ),

                ManifestToolKind::A2a(a2a) => ResolvedContextToolKind::A2a(
                    ResolvedContextToolA2a {
                        url: a2a.url.clone(),
                        auth_token: a2a.auth_token.clone(),
                        headers: a2a.headers.clone(),
                        tenant: match &a2a.tenant {
                            ManifestA2aTenant::Inherit => ResolvedContextToolA2aTenant::Inherit,
                            ManifestA2aTenant::None => ResolvedContextToolA2aTenant::None,
                            ManifestA2aTenant::Fixed { id } => {
                                ResolvedContextToolA2aTenant::Fixed { id: id.clone() }
                            }
                        },
                        timeout_secs: a2a.timeout_secs.clone(),
                        default_accepted_output_modes: a2a
                            .default_accepted_output_modes
                            .clone(),
                    }
                ),

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
                        interpreter: match py.interpreter {
                            ManifestPythonInterpreter::Embedded => ResolvedContextToolPythonInterpreter::Embedded,
                            ManifestPythonInterpreter::Static   => ResolvedContextToolPythonInterpreter::Static,
                        },
                    })
                }

                ManifestToolKind::Bash(bash) => ResolvedContextToolKind::Bash(
                    ResolvedContextToolBash {
                        commands: bash.commands.clone(),
                        fs: ResolvedContextToolBashFs {
                            kind: match &bash.fs.kind {
                                ManifestBashFsKind::InMemory  => ResolvedContextToolBashFsKind::InMemory,
                                ManifestBashFsKind::Overlay   => ResolvedContextToolBashFsKind::Overlay(
                                    bash.fs.path.clone().ok_or_else(|| ManifestError::resolution(
                                        format!("tool `{name}`: fs kind `overlay` requires a `path`")
                                    ))?
                                ),
                                ManifestBashFsKind::ReadWrite => ResolvedContextToolBashFsKind::ReadWrite(
                                    bash.fs.path.clone().ok_or_else(|| ManifestError::resolution(
                                        format!("tool `{name}`: fs kind `read_write` requires a `path`")
                                    ))?
                                ),
                            },
                            cwd: bash.fs.cwd.clone(),
                        },
                        env: match &bash.env.kind {
                            ManifestBashEnvKind::Empty   => ResolvedContextToolBashEnv::Empty,
                            ManifestBashEnvKind::Inherit => ResolvedContextToolBashEnv::Inherit,
                            ManifestBashEnvKind::Allow   => ResolvedContextToolBashEnv::Allow(bash.env.vars.clone()),
                            ManifestBashEnvKind::Deny    => ResolvedContextToolBashEnv::Deny(bash.env.vars.clone()),
                        },
                        limits: ResolvedContextToolBashLimits {
                            max_execution_time_secs: bash.limits.max_execution_time_secs.unwrap_or(30),
                            max_output_size:         bash.limits.max_output_size.unwrap_or(10 * 1024 * 1024),
                            max_command_count:       bash.limits.max_command_count.unwrap_or(10_000),
                            max_loop_iterations:     bash.limits.max_loop_iterations.unwrap_or(10_000),
                        },
                        network: ResolvedContextToolBashNetwork {
                            enabled:              bash.network.enabled.unwrap_or(false),
                            allowed_url_prefixes: bash.network.allowed_url_prefixes.clone(),
                            allowed_methods:      bash.network.allowed_methods.iter().cloned().collect(),
                            max_redirects:        bash.network.max_redirects.unwrap_or(0),
                            max_response_size:    bash.network.max_response_size.unwrap_or(10 * 1024 * 1024),
                            network_timeout_secs: bash.network.network_timeout_secs.unwrap_or(30),
                        },
                    }
                ),
            };

            resolved.insert(
                name.clone(),
                ResolvedContextTool {
                    name: name.clone(),
                    description: tool.description.clone(),
                    enabled: tool.enabled.clone(),
                    capabilities: tool.capabilities.clone(),
                    config: tool.config.clone(),
                    kind,
                },
            );
        }

        Ok(resolved)
    }

    fn resolve_skills(
        &self,
        assets: &[TransformedAsset],
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

                    let skill_md_path = skill_md_artifact
                        .to_string_lossy()
                        .to_string();

                    // The skill directory is the parent of the SKILL.md artifact.
                    let skill_dir = skill_md_artifact
                        .parent()
                        .ok_or_else(|| {
                            ManifestError::resolution(format!(
                                "could not determine skill directory for `{name}`."
                            ))
                        })?
                        .to_path_buf();

                    let resources = transformed
                        .artifacts_of("resource")
                        .into_iter()
                        .filter_map(|a| {
                            let path = a.as_path()?;
                            let rel = path
                                .strip_prefix(&skill_dir)
                                .ok()?
                                .to_string_lossy()
                                .to_string();
                            Some((rel, path.to_string_lossy().to_string()))
                        })
                        .collect();

                    ResolvedContextSkillKind::Source(ResolvedContextSkillSource {
                        skill_md_path,
                        resources,
                    })
                }

                ManifestSkill::Content(c) => {
                    ResolvedContextSkillKind::Content(ResolvedContextSkillContent {
                        description: c.description.clone(),
                        content: c.content.clone(),
                        resources: c.resources.clone(),
                    })
                }
            };

            resolved.insert(name.clone(), ResolvedContextSkill { name: name.clone(), kind });
        }

        Ok(resolved)
    }

    fn resolve_http_server(&self) -> Option<ResolvedContextHttpServer> {
        self.http_server
            .as_ref()
            .map(|http| ResolvedContextHttpServer {
                host: http.host.clone(),
                port: http.port.clone(),
                protocols: http
                    .protocol
                    .as_ref()
                    .map_or_else(Vec::new, |p| {
                        vec![
                            p.ag_ui.as_ref().map(|ag_ui| {
                                ResolvedContextHttpServerProtocol::AgUi(
                                    ResolvedContextHttpServerProtocolAgUi {
                                        path: ag_ui.path.clone(),
                                    },
                                )
                            }),
                            p.a2a.as_ref().map(|a2a| {
                                ResolvedContextHttpServerProtocol::A2a(
                                    ResolvedContextHttpServerProtocolA2a {
                                        path: a2a.path.clone(),
                                    },
                                )
                            }),
                        ]
                        .into_iter()
                        .flatten()
                        .collect()
                    }),
            })
    }

    pub async fn resolve(
        self,
        loader: &dyn ResourceLoader,
        assets: &[TransformedAsset],
    ) -> Result<(ResolvedContext, Value), ManifestError> {
        let agent_label = self.resolve_agent_label()?;

        Ok((
            ResolvedContext {
                slug: agent_label
                    .to_lowercase()
                    .replace([' ', '-'], "_"),
                agent_name: agent_label,
                runtime: self.resolve_runtime(),
                providers: self.resolve_providers(),
                agent: self.resolve_agent().await?,
                blocks: self.resolve_blocks(loader).await?,
                tools: self.resolve_tools(assets)?,
                skills: self.resolve_skills(assets)?,
                http_server: self.resolve_http_server(),
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
    use async_trait::async_trait;

    use super::*;
    use agentc_compiler::generator::errors::GeneratorError;
    use crate::parser::SpecFormat;

    struct EmptyLoader;

    #[async_trait]
    impl ResourceLoader for EmptyLoader {
        async fn load(&self, path: &str) -> Result<String, GeneratorError> {
            Err(GeneratorError::resource_not_found(path))
        }
    }

    struct A2aManifestFixture;

    impl A2aManifestFixture {
        fn json() -> &'static str {
            r#"
{
  "build": {
    "archetype": "standalone"
  },
  "providers": {},
  "agent": {
    "assistant": {
      "version": "0.1.0",
      "graph": {
        "type": "react"
      },
      "model": {
        "provider": "anthropic",
        "name": "claude-haiku-4-5"
      }
    }
  },
  "tool": {
    "planner": {
      "kind": "a2a",
      "description": "Delegate planning subtasks.",
      "enabled": {
        "env": "PLANNER_A2A_ENABLED",
        "default": true
      },
      "capabilities": ["a2a:planner"],
      "url": {
        "env": "PLANNER_A2A_URL",
        "default": "https://planner.example.com"
      },
      "auth_token": {
        "env": "PLANNER_A2A_TOKEN",
        "secret": true
      },
      "headers": {
        "X-Agent": "assistant"
      },
      "tenant": {
        "policy": "fixed",
        "id": {
          "env": "PLANNER_A2A_TENANT",
          "default": "tenant-1"
        }
      },
      "timeout_secs": {
        "env": "PLANNER_A2A_TIMEOUT",
        "default": 90
      },
      "default_accepted_output_modes": ["text/plain"]
    }
  }
}
"#
        }

        fn manifest() -> Manifest {
            SpecFormat::json()
                .deserialize_string::<Manifest>(Self::json())
                .expect("manifest should deserialize")
        }
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
        assert!(A2aManifestFixture::manifest().collect_assets().is_empty());
    }
}
