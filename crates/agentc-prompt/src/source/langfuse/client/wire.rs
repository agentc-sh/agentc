// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use serde::Deserialize;

use super::types::{
    ChatMessage, ChatPrompt, ChatPromptItem, MessagePlaceholder, Prompt, PromptMetadata, TextPrompt,
};

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(super) enum WirePrompt {
    Text(WireTextPrompt),
    Chat(WireChatPrompt),
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WirePromptMetadata {
    name: String,
    version: u32,
    config: serde_json::Value,
    labels: Vec<String>,
    tags: Vec<String>,
    commit_message: Option<String>,
    resolution_graph: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub(super) struct WireTextPrompt {
    #[serde(flatten)]
    metadata: WirePromptMetadata,
    prompt: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct WireChatPrompt {
    #[serde(flatten)]
    metadata: WirePromptMetadata,
    prompt: Vec<WireChatPromptItem>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(super) enum WireChatPromptItem {
    Placeholder(WireMessagePlaceholder),
    Message(WireChatMessage),
}

#[derive(Debug, Deserialize)]
pub(super) struct WireChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct WireMessagePlaceholder {
    #[serde(rename = "type")]
    kind: WireMessagePlaceholderKind,
    name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum WireMessagePlaceholderKind {
    Placeholder,
}

impl From<WirePrompt> for Prompt {
    fn from(value: WirePrompt) -> Self {
        match value {
            WirePrompt::Text(prompt) => Self::Text(prompt.into()),
            WirePrompt::Chat(prompt) => Self::Chat(prompt.into()),
        }
    }
}

impl From<WirePromptMetadata> for PromptMetadata {
    fn from(value: WirePromptMetadata) -> Self {
        Self {
            name: value.name,
            version: value.version,
            config: value.config,
            labels: value.labels,
            tags: value.tags,
            commit_message: value.commit_message,
            resolution_graph: value.resolution_graph,
        }
    }
}

impl From<WireTextPrompt> for TextPrompt {
    fn from(value: WireTextPrompt) -> Self {
        Self {
            metadata: value.metadata.into(),
            prompt: value.prompt,
        }
    }
}

impl From<WireChatPrompt> for ChatPrompt {
    fn from(value: WireChatPrompt) -> Self {
        Self {
            metadata: value.metadata.into(),
            prompt: value
                .prompt
                .into_iter()
                .map(ChatPromptItem::from)
                .collect(),
        }
    }
}

impl From<WireChatPromptItem> for ChatPromptItem {
    fn from(value: WireChatPromptItem) -> Self {
        match value {
            WireChatPromptItem::Message(message) => Self::Message(ChatMessage {
                role: message.role,
                content: message.content,
            }),
            WireChatPromptItem::Placeholder(placeholder) => {
                match placeholder.kind {
                    WireMessagePlaceholderKind::Placeholder => {}
                }

                Self::Placeholder(MessagePlaceholder { name: placeholder.name })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    struct WireFixture;

    impl WireFixture {
        fn metadata() -> serde_json::Value {
            json!({
                "name": "support/assistant",
                "version": 7,
                "config": {"temperature": 0.2},
                "labels": ["production"],
                "tags": ["support"],
                "commitMessage": "Publish support prompt",
                "resolutionGraph": {"root": 7},
            })
        }

        fn decode(prompt: serde_json::Value) -> Prompt {
            serde_json::from_value::<WirePrompt>(prompt)
                .expect("wire prompt should decode")
                .into()
        }
    }

    #[test]
    fn text_payload_preserves_content_and_metadata() {
        let mut payload = WireFixture::metadata();
        payload["type"] = json!("text");
        payload["prompt"] = json!("You are {{ agent_name }}.");

        assert!(matches!(
            WireFixture::decode(payload),
            Prompt::Text(prompt)
                if prompt.prompt == "You are {{ agent_name }}."
                    && prompt.metadata.name == "support/assistant"
                    && prompt.metadata.version == 7
                    && prompt.metadata.config == json!({"temperature": 0.2})
                    && prompt.metadata.labels == vec!["production"]
                    && prompt.metadata.tags == vec!["support"]
                    && prompt.metadata.commit_message.as_deref()
                        == Some("Publish support prompt")
                    && prompt.metadata.resolution_graph == Some(json!({"root": 7}))
        ));
    }

    #[test]
    fn chat_payload_preserves_message_and_placeholder_order() {
        let mut payload = WireFixture::metadata();
        payload["type"] = json!("chat");
        payload["prompt"] = json!([
            {
                "role": "system",
                "content": "You are helpful.",
            },
            {
                "type": "placeholder",
                "name": "chat_history",
            },
            {
                "role": "user",
                "content": "Be concise.",
            },
        ]);

        assert!(matches!(
            WireFixture::decode(payload),
            Prompt::Chat(prompt)
                if matches!(
                    &prompt.prompt[..],
                    [
                        ChatPromptItem::Message(ChatMessage { role, content }),
                        ChatPromptItem::Placeholder(MessagePlaceholder { name, .. }),
                        ChatPromptItem::Message(ChatMessage {
                            role: user_role,
                            content: user_content,
                        }),
                    ] if role == "system"
                        && content == "You are helpful."
                        && name == "chat_history"
                        && user_role == "user"
                        && user_content == "Be concise."
                )
        ));
    }
}
