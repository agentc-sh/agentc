// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::protocol::{
    ids::{MessageId, ToolCallId},
    tool::ToolCall,
};

/// Message role.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Developer,
    System,
    Assistant,
    User,
    Tool,
    Reasoning,
}

// Utility methods for serde defaults
impl Role {
    pub(crate) fn developer() -> Self {
        Self::Developer
    }
    pub(crate) fn system() -> Self {
        Self::System
    }
    pub(crate) fn assistant() -> Self {
        Self::Assistant
    }
    pub(crate) fn user() -> Self {
        Self::User
    }
    pub(crate) fn tool() -> Self {
        Self::Tool
    }
    pub(crate) fn reasoning() -> Self {
        Self::Reasoning
    }
}

/// Represents the different type of messages that you might receive, but as an enum.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(tag = "role", rename_all = "lowercase")]
pub enum Message {
    Developer {
        #[serde(default = "MessageId::random")]
        id: MessageId,
        content: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    System {
        #[serde(default = "MessageId::random")]
        id: MessageId,
        content: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    Assistant {
        #[serde(default = "MessageId::random")]
        id: MessageId,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        content: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        #[serde(rename = "toolCalls", default, skip_serializing_if = "Option::is_none")]
        tool_calls: Option<Vec<ToolCall>>,
    },
    User {
        #[serde(default = "MessageId::random")]
        id: MessageId,
        content: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    Tool {
        #[serde(default = "MessageId::random")]
        id: MessageId,
        content: String,
        #[serde(rename = "toolCallId")]
        tool_call_id: ToolCallId,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },
    Reasoning {
        #[serde(default = "MessageId::random")]
        id: MessageId,
        content: String,
        /// Encrypted chain-of-thought blob stored and forwarded opaquely by the client.
        #[serde(
            rename = "encryptedValue",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        encrypted_value: Option<String>,
    },
}

impl Message {
    pub fn new<S: AsRef<str>>(role: Role, id: impl Into<MessageId>, content: S) -> Self {
        match role {
            Role::Developer => Self::Developer {
                id: id.into(),
                content: content.as_ref().to_string(),
                name: None,
            },
            Role::System => Self::System {
                id: id.into(),
                content: content.as_ref().to_string(),
                name: None,
            },
            Role::Assistant => Self::Assistant {
                id: id.into(),
                content: Some(content.as_ref().to_string()),
                name: None,
                tool_calls: None,
            },
            Role::User => Self::User {
                id: id.into(),
                content: content.as_ref().to_string(),
                name: None,
            },
            Role::Tool => Self::Tool {
                id: id.into(),
                content: content.as_ref().to_string(),
                tool_call_id: ToolCallId::random(),
                error: None,
            },
            Role::Reasoning => Self::Reasoning {
                id: id.into(),
                content: content.as_ref().to_string(),
                encrypted_value: None,
            },
        }
    }

    /// Returns a User message with a random ID and the given content
    pub fn new_user<S: AsRef<str>>(content: S) -> Self {
        Self::new(Role::user(), MessageId::random(), content)
    }

    /// Returns a Tool message with a random ID and the given content
    pub fn new_tool<S: AsRef<str>>(content: S) -> Self {
        Self::new(Role::tool(), MessageId::random(), content)
    }

    /// Returns a System message with a random ID and the given content
    pub fn new_system<S: AsRef<str>>(content: S) -> Self {
        Self::new(Role::system(), MessageId::random(), content)
    }

    /// Returns an Assistant message with a random ID and the given content
    pub fn new_assistant<S: AsRef<str>>(content: S) -> Self {
        Self::new(Role::assistant(), MessageId::random(), content)
    }

    /// Returns a Developer message with a random ID and the given content
    pub fn new_developer<S: AsRef<str>>(content: S) -> Self {
        Self::new(Role::developer(), MessageId::random(), content)
    }

    pub fn id(&self) -> &MessageId {
        match self {
            Message::Developer { id, .. } => id,
            Message::System { id, .. } => id,
            Message::Assistant { id, .. } => id,
            Message::User { id, .. } => id,
            Message::Tool { id, .. } => id,
            Message::Reasoning { id, .. } => id,
        }
    }

    pub fn id_mut(&mut self) -> &mut MessageId {
        match self {
            Message::Developer { id, .. } => id,
            Message::System { id, .. } => id,
            Message::Assistant { id, .. } => id,
            Message::User { id, .. } => id,
            Message::Tool { id, .. } => id,
            Message::Reasoning { id, .. } => id,
        }
    }

    pub fn role(&self) -> Role {
        match self {
            Message::Developer { .. } => Role::developer(),
            Message::System { .. } => Role::system(),
            Message::Assistant { .. } => Role::assistant(),
            Message::User { .. } => Role::user(),
            Message::Tool { .. } => Role::tool(),
            Message::Reasoning { .. } => Role::reasoning(),
        }
    }

    pub fn content(&self) -> Option<&str> {
        match self {
            Message::Developer { content, .. } => Some(content),
            Message::System { content, .. } => Some(content),
            Message::User { content, .. } => Some(content),
            Message::Tool { content, .. } => Some(content),
            Message::Reasoning { content, .. } => Some(content),
            Message::Assistant { content, .. } => content.as_deref(),
        }
    }

    pub fn content_mut(&mut self) -> Option<&mut String> {
        match self {
            Message::Developer { content, .. }
            | Message::System { content, .. }
            | Message::User { content, .. }
            | Message::Tool { content, .. }
            | Message::Reasoning { content, .. } => Some(content),
            Message::Assistant { content, .. } => {
                if content.is_none() {
                    *content = Some(String::new());
                }
                content.as_mut()
            }
        }
    }

    pub fn tool_calls(&self) -> Option<&[ToolCall]> {
        match self {
            Message::Assistant { tool_calls, .. } => tool_calls.as_deref(),
            _ => None,
        }
    }

    pub fn tool_calls_mut(&mut self) -> Option<&mut Vec<ToolCall>> {
        match self {
            Message::Assistant { tool_calls, .. } => {
                if tool_calls.is_none() {
                    *tool_calls = Some(Vec::new());
                }
                tool_calls.as_mut()
            }
            _ => None,
        }
    }
}
