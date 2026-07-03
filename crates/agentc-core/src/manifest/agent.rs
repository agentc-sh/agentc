// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use sanitizer::prelude::*;
use serde::{Deserialize, Serialize};
use validator::Validate;

use agentc_blocks::types::RuntimeValue;

#[derive(Debug, Clone, Serialize, Deserialize, Validate, Sanitizer)]
pub struct ManifestAgent {
    /// Semver version string for this agent.
    #[serde(default = "default_version")]
    #[validate(length(min = 1))]
    #[sanitizer(trim)]
    pub version: String,
    /// Optional description of the agent.
    #[serde(default)]
    #[validate(length(min = 1))]
    #[sanitizer(trim)]
    pub description: Option<String>,
    /// The prompt template to use for this agent.
    #[serde(default)]
    pub prompt: Option<ManifestAgentPrompt>,
    /// The capabilities of the agent.
    #[serde(default)]
    pub capabilities: Option<RuntimeValue<Vec<String>>>,
    /// The capability policy to apply when using this agent.
    #[serde(default)]
    pub capability_policy: Option<RuntimeValue<String>>,
    /// The default model configuration.
    #[validate(nested)]
    pub model: ManifestAgentModel,
}

fn default_version() -> String {
    "0.1.0".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ManifestAgentPrompt {
    /// A simple string prompt template which gets sent as a system message.
    Prompt(String),
    /// A list of messages with roles, allowing for more complex prompts.
    Messages(Vec<ManifestAgentPromptMessage>),
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, Sanitizer)]
pub struct ManifestAgentPromptMessage {
    /// The role of the message (system, user, assistant).
    pub role: ManifestAgentPromptMessageRole,
    /// The content of the message, which is a jinja2 template string.
    #[validate(length(min = 1))]
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManifestAgentPromptMessageRole {
    System,
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, Sanitizer)]
pub struct ManifestAgentModel {
    /// The provider of the model to use (e.g., "openai", "anthropic", "custom").
    pub provider: RuntimeValue<String>,
    /// The name of the model to use (e.g., "gpt-4", "claude-2", "my-custom-model").
    pub name: RuntimeValue<String>,
}
