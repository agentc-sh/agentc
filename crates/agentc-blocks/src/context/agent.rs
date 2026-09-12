// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};

use crate::types::RuntimeValue;

/// Resolved agent context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedContextAgent {
    /// Semver version string for this agent.
    pub version: String,
    /// Optional description of the agent.
    pub description: Option<String>,
    /// The resolved prompt source. `None` means an empty constant prompt.
    pub prompt: Option<ResolvedContextAgentPromptSource>,
    /// The capabilities of the agent.
    pub capabilities: Option<RuntimeValue<Vec<String>>>,
    /// The capability policy to apply when using this agent.
    pub capability_policy: Option<RuntimeValue<String>>,
    /// The default model configuration.
    pub model: ResolvedContextAgentModel,
}

/// How an agent's prompt is sourced.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolvedContextAgentPromptSource {
    Constant {
        messages: Vec<ResolvedContextAgentPromptMessage>,
    },
    Langfuse(ResolvedContextAgentPromptSourceLangfuse),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedContextAgentPromptSourceLangfuse {
    pub prompt_name: RuntimeValue<String>,
    pub public_key: RuntimeValue<String>,
    pub secret_key: RuntimeValue<String>,
    pub base_url: Option<RuntimeValue<String>>,
    pub label: Option<RuntimeValue<String>>,
    pub version: Option<RuntimeValue<u32>>,
    pub cache_ttl_seconds: Option<RuntimeValue<u64>>,
    pub fetch_timeout_seconds: Option<RuntimeValue<u64>>,
    pub max_retries: Option<RuntimeValue<u32>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedContextAgentPromptMessage {
    /// The role of the message (system, user, assistant).
    pub role: ResolvedContextAgentPromptMessageRole,
    /// The content of the message, which is a jinja2 template string.
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolvedContextAgentPromptMessageRole {
    System,
    User,
    Assistant,
}

/// Resolved model context for an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedContextAgentModel {
    /// The provider of the model to use (e.g., "openai", "anthropic", "custom").
    pub provider: RuntimeValue<String>,
    /// The name of the model to use (e.g., "gpt-4", "claude-2", "my-custom-model").
    pub name: RuntimeValue<String>,
}
