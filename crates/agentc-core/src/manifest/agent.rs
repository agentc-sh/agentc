// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use sanitizer::prelude::*;
use serde::{Deserialize, Serialize};
use validator::Validate;

use agentc_blocks::types::RuntimeValue;

use crate::manifest::ManifestAgentGraph;

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
    /// The graph implementation used by this agent.
    pub graph: ManifestAgentGraph,
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
    /// A runtime-backed prompt source.
    Source(ManifestAgentPromptSource),
}

/// A configured external prompt source.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "source", rename_all = "snake_case")]
pub enum ManifestAgentPromptSource {
    /// Retrieves the prompt from Langfuse Prompt Management.
    Langfuse(ManifestAgentPromptSourceLangfuse),
}

/// Manifest configuration for a Langfuse prompt source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestAgentPromptSourceLangfuse {
    /// The Langfuse prompt name, including any folder path.
    pub prompt_name: RuntimeValue<String>,
    /// The Langfuse project public key.
    pub public_key: RuntimeValue<String>,
    /// The Langfuse project secret key.
    pub secret_key: RuntimeValue<String>,
    /// The optional Langfuse Cloud or self-hosted base URL.
    #[serde(default)]
    pub base_url: Option<RuntimeValue<String>>,
    /// The optional movable label used to select a prompt version.
    #[serde(default)]
    pub label: Option<RuntimeValue<String>>,
    /// The optional immutable numeric prompt version.
    #[serde(default)]
    pub version: Option<RuntimeValue<u32>>,
    /// The optional local prompt cache lifetime in seconds.
    #[serde(default)]
    pub cache_ttl_seconds: Option<RuntimeValue<u64>>,
    /// The optional prompt fetch timeout in seconds.
    #[serde(default)]
    pub fetch_timeout_seconds: Option<RuntimeValue<u64>>,
    /// The optional number of additional prompt fetch retries.
    #[serde(default)]
    pub max_retries: Option<RuntimeValue<u32>>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        manifest::Manifest,
        parser::{
            SpecFormat,
            middleware::hcl::RuntimeFunctionDeserialize,
        },
    };

    struct AgentPromptFixture;

    impl AgentPromptFixture {
        fn parse(prompt: &str) -> ManifestAgentPrompt {
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

  prompt = {prompt}

  model {{
    provider = "anthropic"
    name     = "claude-haiku-4-5"
  }}
}}
"#
                ))
                .expect("manifest should deserialize")
                .agent
                .remove("assistant")
                .expect("assistant should exist")
                .prompt
                .expect("prompt should exist")
        }
    }

    #[test]
    fn parses_string_prompt() {
        assert!(matches!(
            AgentPromptFixture::parse(r#""You are helpful.""#),
            ManifestAgentPrompt::Prompt(prompt) if prompt == "You are helpful."
        ));
    }

    #[test]
    fn parses_message_prompt() {
        let ManifestAgentPrompt::Messages(messages) = AgentPromptFixture::parse(
            r#"[
    { role = "system", content = "System" },
    { role = "user", content = "User" }
  ]"#,
        ) else {
            panic!("prompt should contain messages");
        };

        assert_eq!(messages.len(), 2);
        assert!(matches!(
            &messages[0].role,
            ManifestAgentPromptMessageRole::System
        ));
        assert_eq!(messages[0].content, "System");
        assert!(matches!(
            &messages[1].role,
            ManifestAgentPromptMessageRole::User
        ));
        assert_eq!(messages[1].content, "User");
    }

    #[test]
    fn parses_langfuse_runtime_configuration() {
        let ManifestAgentPrompt::Source(ManifestAgentPromptSource::Langfuse(prompt)) =
            AgentPromptFixture::parse(
                r#"{
    source = "langfuse"

    prompt_name           = runtime("LANGFUSE_PROMPT_NAME")
    public_key            = runtime("LANGFUSE_PUBLIC_KEY")
    secret_key            = secret(runtime("LANGFUSE_SECRET_KEY"))
    base_url              = runtime("LANGFUSE_BASE_URL")
    label                 = runtime("LANGFUSE_LABEL")
    cache_ttl_seconds     = runtime("LANGFUSE_CACHE_TTL", 30)
    fetch_timeout_seconds = runtime("LANGFUSE_FETCH_TIMEOUT", 5)
    max_retries           = runtime("LANGFUSE_MAX_RETRIES", 2)
  }"#,
            )
        else {
            panic!("prompt should be a Langfuse source");
        };

        assert!(matches!(
            prompt.prompt_name,
            RuntimeValue::Runtime { env, default: None, secret: false }
                if env == "LANGFUSE_PROMPT_NAME"
        ));
        assert!(matches!(
            prompt.public_key,
            RuntimeValue::Runtime { env, default: None, secret: false }
                if env == "LANGFUSE_PUBLIC_KEY"
        ));
        assert!(matches!(
            prompt.secret_key,
            RuntimeValue::Runtime { env, default: None, secret: true }
                if env == "LANGFUSE_SECRET_KEY"
        ));
        assert!(matches!(
            prompt.base_url,
            Some(RuntimeValue::Runtime { env, default: None, secret: false })
                if env == "LANGFUSE_BASE_URL"
        ));
        assert!(matches!(
            prompt.label,
            Some(RuntimeValue::Runtime { env, default: None, secret: false })
                if env == "LANGFUSE_LABEL"
        ));
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

    #[test]
    fn parses_langfuse_version_selector() {
        let ManifestAgentPrompt::Source(ManifestAgentPromptSource::Langfuse(prompt)) =
            AgentPromptFixture::parse(
                r#"{
    source = "langfuse"

    prompt_name = "support/assistant"
    public_key  = runtime("LANGFUSE_PUBLIC_KEY")
    secret_key  = secret(runtime("LANGFUSE_SECRET_KEY"))
    version     = 7
  }"#,
            )
        else {
            panic!("prompt should be a Langfuse source");
        };

        assert!(matches!(
            prompt.version,
            Some(RuntimeValue::Constant(7))
        ));
    }
}
