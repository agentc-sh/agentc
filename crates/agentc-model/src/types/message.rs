// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};

use crate::{
    errors::ModelError,
    types::{
        media::{Audio, Document, Image, Video},
        reasoning::Reasoning,
        tools::{ToolCall, ToolResult, ToolResultContent},
    },
};

/// A single message in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "role", rename_all = "snake_case")]
pub enum ChatMessage {
    /// A system prompt message, used to prime the model with instructions or context.
    System(SystemMessage),
    /// A user turn message, representing input from the user.
    User(UserMessage),
    /// An assistant turn message, representing output from the model.
    Assistant(AssistantMessage),
}

impl ChatMessage {
    pub fn role(&self) -> &str {
        match self {
            ChatMessage::System(_) => "system",
            ChatMessage::User(_) => "user",
            ChatMessage::Assistant(_) => "assistant",
        }
    }

    pub fn system(content: impl Into<String>) -> Self {
        Self::System(SystemMessage { content: content.into() })
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self::User(UserMessage {
            content: vec![UserContent::Text(content.into())],
        })
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self::Assistant(AssistantMessage {
            id: None,
            content: vec![AssistantContent::Text(content.into())],
        })
    }

    pub fn assistant_with_id(id: impl Into<String>, content: impl Into<String>) -> Self {
        Self::Assistant(AssistantMessage {
            id: Some(id.into()),
            content: vec![AssistantContent::Text(content.into())],
        })
    }

    pub fn tool_result(call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self::User(UserMessage {
            content: vec![UserContent::ToolResult(ToolResult {
                call_id: call_id.into(),
                content: vec![ToolResultContent::Text(content.into())],
            })],
        })
    }
}

/// A system prompt message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMessage {
    pub content: String,
}

/// A user turn message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMessage {
    pub content: Vec<UserContent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum UserContent {
    Text(String),
    ToolResult(ToolResult),
    Image(Image),
    Audio(Audio),
    Video(Video),
    Document(Document),
}

/// An assistant turn message, optionally containing tool calls.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantMessage {
    pub id: Option<String>,
    pub content: Vec<AssistantContent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AssistantContent {
    Text(String),
    ToolCall(ToolCall),
    Reasoning(Reasoning),
    Image(Image),
}

/// A conversation history, consisting of a sequence of chat messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatHistory(Vec<ChatMessage>);

impl ChatHistory {
    /// Creates a new chat history from a vector of messages.
    pub fn new(messages: Vec<ChatMessage>) -> Self {
        Self(messages)
    }

    /// Returns a reference to the messages in the chat history.
    pub fn messages(&self) -> &[ChatMessage] {
        &self.0
    }

    /// Returns a mutable reference to the messages in the chat history.
    pub fn messages_mut(&mut self) -> &mut Vec<ChatMessage> {
        &mut self.0
    }

    /// Consumes the chat history and returns the messages as a vector.
    pub fn into_messages(self) -> Vec<ChatMessage> {
        self.0
    }

    /// Adds a message to the end of the chat history.
    pub fn push(&mut self, message: ChatMessage) {
        self.0.push(message);
    }

    /// Extends the chat history with a vector of messages.
    pub fn extend(&mut self, messages: Vec<ChatMessage>) {
        self.0.extend(messages);
    }

    /// Clears all messages from the chat history.
    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// Removes the message at the specified index from the chat history and returns it.
    pub fn remove(&mut self, index: usize) -> Option<ChatMessage> {
        if index < self.0.len() {
            Some(self.0.remove(index))
        } else {
            None
        }
    }

    /// Removes the last message from the chat history and returns it.
    pub fn pop(&mut self) -> Option<ChatMessage> {
        self.0.pop()
    }

    /// Splits the chat history into an optional system prompt, the latest user message, and the remaining messages.
    /// All system messages are extracted and their content concatenated into a single preamble string so that
    /// none remain in the returned history (providers represent system context as a preamble, not a turn).
    pub fn split(mut self) -> Result<(Option<String>, UserMessage, Vec<ChatMessage>), ModelError> {
        let mut system_parts = Vec::new();
        self.0.retain(|m| {
            if let ChatMessage::System(s) = m {
                system_parts.push(s.content.clone());
                false
            } else {
                true
            }
        });

        Ok((
            (!system_parts.is_empty()).then(|| system_parts.join("\n")),
            match self.pop() {
                Some(ChatMessage::User(content)) => content,
                Some(_) => {
                    return Err(ModelError::invalid_request(
                        "latest message must be a user message",
                    ));
                }
                None => return Err(ModelError::invalid_request("no messages in request")),
            },
            self.into_messages(),
        ))
    }
}
