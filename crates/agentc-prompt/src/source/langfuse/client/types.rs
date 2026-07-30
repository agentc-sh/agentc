// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::{
    errors::PromptError,
    template::{PromptTemplate, Role},
};

/// Selects a Langfuse prompt version.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum PromptSelector {
    /// Uses Langfuse's default production selection.
    #[default]
    Default,
    /// Uses the prompt version carrying the given label.
    Label(String),
    /// Uses an immutable numeric prompt version.
    Version(u32),
}

/// Controls cache reuse for one prompt retrieval.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum PromptCacheMode {
    /// Uses the client's configured prompt cache lifetime.
    #[default]
    Default,
    /// Bypasses the prompt cache.
    Disabled,
    /// Uses the provided cache lifetime.
    TimeToLive(Duration),
}

/// Options for retrieving a Langfuse prompt.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GetPromptRequest {
    pub selector: PromptSelector,
    pub cache: PromptCacheMode,
}

impl GetPromptRequest {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.selector = PromptSelector::Label(label.into());
        self
    }

    pub fn with_version(mut self, version: u32) -> Self {
        self.selector = PromptSelector::Version(version);
        self
    }

    pub fn with_cache(mut self, cache: PromptCacheMode) -> Self {
        self.cache = cache;
        self
    }

    pub fn without_cache(mut self) -> Self {
        self.cache = PromptCacheMode::Disabled;
        self
    }

    pub fn with_cache_ttl(mut self, ttl: Duration) -> Self {
        self.cache = if ttl.is_zero() {
            PromptCacheMode::Disabled
        } else {
            PromptCacheMode::TimeToLive(ttl)
        };
        self
    }
}

/// A prompt retrieved from Langfuse.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Prompt {
    Text(TextPrompt),
    Chat(ChatPrompt),
}

impl Prompt {
    pub fn metadata(&self) -> &PromptMetadata {
        match self {
            Self::Text(prompt) => &prompt.metadata,
            Self::Chat(prompt) => &prompt.metadata,
        }
    }

    pub fn name(&self) -> &str {
        &self.metadata().name
    }

    pub fn version(&self) -> u32 {
        self.metadata().version
    }
}

impl TryFrom<Prompt> for PromptTemplate {
    type Error = PromptError;

    fn try_from(value: Prompt) -> Result<Self, Self::Error> {
        match value {
            Prompt::Text(prompt) => Ok(Self::system(prompt.prompt)),
            Prompt::Chat(prompt) => prompt.try_into(),
        }
    }
}

/// Metadata shared by text and chat prompts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptMetadata {
    pub name: String,
    pub version: u32,
    pub config: serde_json::Value,
    pub labels: Vec<String>,
    pub tags: Vec<String>,
    pub commit_message: Option<String>,
    pub resolution_graph: Option<serde_json::Value>,
}

/// A Langfuse text prompt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextPrompt {
    #[serde(flatten)]
    pub metadata: PromptMetadata,
    pub prompt: String,
}

/// A Langfuse chat prompt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatPrompt {
    #[serde(flatten)]
    pub metadata: PromptMetadata,
    pub prompt: Vec<ChatPromptItem>,
}

impl TryFrom<ChatPrompt> for PromptTemplate {
    type Error = PromptError;

    fn try_from(value: ChatPrompt) -> Result<Self, Self::Error> {
        let mut template = Self::new();

        for item in value.prompt {
            match item {
                ChatPromptItem::Message(message) => {
                    template = template.with_part(
                        Role::try_from(message.role.as_str())?,
                        message.content,
                    );
                }
                ChatPromptItem::Placeholder(placeholder) => {
                    return Err(PromptError::source(format!(
                        "Langfuse message placeholder `{}` is not supported",
                        placeholder.name,
                    )));
                }
            }
        }

        Ok(template)
    }
}

/// An item in a Langfuse chat prompt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ChatPromptItem {
    Message(ChatMessage),
    Placeholder(MessagePlaceholder),
}

/// A role and content message in a Langfuse chat prompt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// A structural message placeholder in a Langfuse chat prompt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessagePlaceholder {
    pub name: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct PromptFixture;

    impl PromptFixture {
        fn metadata() -> PromptMetadata {
            PromptMetadata {
                name: "assistant".to_string(),
                version: 3,
                config: json!({}),
                labels: vec!["production".to_string()],
                tags: vec![],
                commit_message: None,
                resolution_graph: None,
            }
        }

        fn text(prompt: impl Into<String>) -> Prompt {
            Prompt::Text(TextPrompt {
                metadata: Self::metadata(),
                prompt: prompt.into(),
            })
        }

        fn chat(items: Vec<ChatPromptItem>) -> Prompt {
            Prompt::Chat(ChatPrompt {
                metadata: Self::metadata(),
                prompt: items,
            })
        }

        fn message(role: &str, content: &str) -> ChatPromptItem {
            ChatPromptItem::Message(ChatMessage {
                role: role.to_string(),
                content: content.to_string(),
            })
        }
    }

    #[test]
    fn request_defaults_to_provider_selection_and_client_cache() {
        assert_eq!(
            GetPromptRequest::new(),
            GetPromptRequest {
                selector: PromptSelector::Default,
                cache: PromptCacheMode::Default,
            },
        );
    }

    #[test]
    fn request_fluent_methods_set_selection_and_cache() {
        assert_eq!(
            GetPromptRequest::new()
                .with_label("staging")
                .with_cache(PromptCacheMode::TimeToLive(Duration::from_secs(30))),
            GetPromptRequest {
                selector: PromptSelector::Label("staging".to_string()),
                cache: PromptCacheMode::TimeToLive(Duration::from_secs(30)),
            },
        );
        assert_eq!(
            GetPromptRequest::new()
                .with_version(7)
                .without_cache(),
            GetPromptRequest {
                selector: PromptSelector::Version(7),
                cache: PromptCacheMode::Disabled,
            },
        );
    }

    #[test]
    fn zero_cache_ttl_disables_caching() {
        assert_eq!(
            GetPromptRequest::new()
                .with_cache_ttl(Duration::ZERO)
                .cache,
            PromptCacheMode::Disabled,
        );
    }

    #[test]
    fn text_prompt_converts_to_one_system_part_without_rendering_jinja() {
        assert_eq!(
            PromptTemplate::try_from(
                PromptFixture::text("You are {{ agent_name }}."),
            )
            .expect("text prompt should convert")
            .into_parts()
            .collect::<Vec<_>>(),
            vec![(Role::System, "You are {{ agent_name }}.".to_string())],
        );
    }

    #[test]
    fn chat_prompt_converts_messages_in_order() {
        assert_eq!(
            PromptTemplate::try_from(PromptFixture::chat(vec![
                PromptFixture::message("system", "System"),
                PromptFixture::message("user", "User"),
                PromptFixture::message("assistant", "Assistant"),
            ]))
            .expect("chat prompt should convert")
            .into_parts()
            .collect::<Vec<_>>(),
            vec![
                (Role::System, "System".to_string()),
                (Role::User, "User".to_string()),
                (Role::Assistant, "Assistant".to_string()),
            ],
        );
    }

    #[test]
    fn empty_text_and_chat_prompts_convert() {
        assert_eq!(
            PromptTemplate::try_from(PromptFixture::text(""))
                .expect("empty text prompt should convert")
                .into_parts()
                .collect::<Vec<_>>(),
            vec![(Role::System, String::new())],
        );
        assert!(
            PromptTemplate::try_from(PromptFixture::chat(vec![]))
                .expect("empty chat prompt should convert")
                .into_parts()
                .next()
                .is_none()
        );
    }

    #[test]
    fn chat_prompt_rejects_unknown_roles_and_placeholders() {
        assert!(matches!(
            PromptTemplate::try_from(PromptFixture::chat(vec![
                PromptFixture::message("tool", "Result"),
            ])),
            Err(PromptError::Source { message, .. }) if message.contains("tool")
        ));
        assert!(matches!(
            PromptTemplate::try_from(PromptFixture::chat(vec![
                ChatPromptItem::Placeholder(MessagePlaceholder {
                    name: "chat_history".to_string(),
                }),
            ])),
            Err(PromptError::Source { message, .. })
                if message.contains("chat_history")
        ));
    }
}
