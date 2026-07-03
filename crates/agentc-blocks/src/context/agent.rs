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
    /// The prompt template to use for this agent.
    pub prompt: Option<Vec<ResolvedContextAgentPromptMessage>>,
    /// The capabilities of the agent.
    pub capabilities: Option<RuntimeValue<Vec<String>>>,
    /// The capability policy to apply when using this agent.
    pub capability_policy: Option<RuntimeValue<String>>,
    /// The default model configuration.
    pub model: ResolvedContextAgentModel,
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
