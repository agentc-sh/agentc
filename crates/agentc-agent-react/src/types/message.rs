// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    iter::FromIterator,
    str::FromStr,
};
use uuid::Uuid;

use agentc_agent::{
    context::AgentContext,
    types::{
        conversion::{FromModelType, ToModelType},
        tools::ToolCall,
    },
};
use agentc_model::types::{
    message::{
        AssistantContent as ModelAssistantContent, AssistantMessage as ModelAssistantMessage,
        ChatMessage as ModelChatMessage, SystemMessage as ModelSystemMessage,
        UserContent as ModelUserContent, UserMessage as ModelUserMessage,
    },
    reasoning::{Reasoning as ModelReasoning, ReasoningContent as ModelReasoningContent},
    tools::{ToolResult as ModelToolResult, ToolResultContent as ModelToolResultContent},
};
use agentc_prompt::{buffer::TokenCount, compaction::MessageGroup};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
    Reasoning,
}

impl MessageRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            MessageRole::System => "system",
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::Tool => "tool",
            MessageRole::Reasoning => "reasoning",
        }
    }
}

impl FromStr for MessageRole {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "system" => Ok(MessageRole::System),
            "user" => Ok(MessageRole::User),
            "assistant" => Ok(MessageRole::Assistant),
            "tool" => Ok(MessageRole::Tool),
            "reasoning" => Ok(MessageRole::Reasoning),
            _ => Err(()),
        }
    }
}

impl From<String> for MessageRole {
    fn from(s: String) -> Self {
        MessageRole::from_str(&s).unwrap_or(MessageRole::User)
    }
}

impl From<&str> for MessageRole {
    fn from(s: &str) -> Self {
        MessageRole::from_str(s).unwrap_or(MessageRole::User)
    }
}

impl Display for MessageRole {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.as_str())
    }
}

/// A system message.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SystemMessage {
    pub id: Uuid,
    pub tenant_id: String,
    pub session_id: Uuid,
    pub run_id: Option<Uuid>,
    pub content: String,
    pub name: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl SystemMessage {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            tenant_id: String::new(),
            session_id: Uuid::nil(),
            run_id: None,
            content: content.into(),
            name: None,
            created_at: Utc::now(),
        }
    }

    pub fn id(&self) -> &Uuid {
        &self.id
    }
    pub fn role(&self) -> &MessageRole {
        &MessageRole::System
    }

    pub fn with_id(mut self, id: impl Into<Uuid>) -> Self {
        self.id = id.into();
        self
    }

    pub fn with_tenant_id(mut self, tenant_id: impl Into<String>) -> Self {
        self.tenant_id = tenant_id.into();
        self
    }

    pub fn with_session_id(mut self, session_id: impl Into<Uuid>) -> Self {
        self.session_id = session_id.into();
        self
    }

    pub fn with_run_id(mut self, run_id: impl Into<Uuid>) -> Self {
        self.run_id = Some(run_id.into());
        self
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_created_at(mut self, created_at: impl Into<DateTime<Utc>>) -> Self {
        self.created_at = created_at.into();
        self
    }
}

impl ToModelType for SystemMessage {
    type ModelType = ModelSystemMessage;

    fn to_model_type(&self) -> Self::ModelType {
        ModelSystemMessage { content: self.content.clone() }
    }
}

impl FromModelType for SystemMessage {
    type ModelType = ModelSystemMessage;
    type Output = Self;

    fn from_model_type(model: Self::ModelType) -> Self::Output {
        Self::new(model.content)
    }
}

/// A user message.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserMessage {
    pub id: Uuid,
    pub tenant_id: String,
    pub session_id: Uuid,
    pub run_id: Option<Uuid>,
    pub content: String,
    pub name: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl UserMessage {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            tenant_id: String::new(),
            session_id: Uuid::nil(),
            run_id: None,
            content: content.into(),
            name: None,
            created_at: Utc::now(),
        }
    }

    pub fn id(&self) -> &Uuid {
        &self.id
    }
    pub fn role(&self) -> &MessageRole {
        &MessageRole::User
    }

    pub fn with_id(mut self, id: impl Into<Uuid>) -> Self {
        self.id = id.into();
        self
    }

    pub fn with_tenant_id(mut self, tenant_id: impl Into<String>) -> Self {
        self.tenant_id = tenant_id.into();
        self
    }

    pub fn with_session_id(mut self, session_id: impl Into<Uuid>) -> Self {
        self.session_id = session_id.into();
        self
    }

    pub fn with_run_id(mut self, run_id: impl Into<Uuid>) -> Self {
        self.run_id = Some(run_id.into());
        self
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_created_at(mut self, created_at: impl Into<DateTime<Utc>>) -> Self {
        self.created_at = created_at.into();
        self
    }
}

impl ToModelType for UserMessage {
    type ModelType = ModelUserMessage;

    fn to_model_type(&self) -> Self::ModelType {
        ModelUserMessage {
            content: vec![ModelUserContent::Text(self.content.clone())],
        }
    }
}

impl FromModelType for UserMessage {
    type ModelType = ModelUserMessage;
    type Output = Self;

    fn from_model_type(model: Self::ModelType) -> Self::Output {
        Self::new(
            model
                .content
                .iter()
                .filter_map(|content| match content {
                    ModelUserContent::Text(text) => Some(text.clone()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n"),
        )
    }
}

/// An assistant message.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssistantMessage {
    pub id: Uuid,
    pub tenant_id: String,
    pub session_id: Uuid,
    pub run_id: Uuid,
    pub content: Option<String>,
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    pub created_at: DateTime<Utc>,
}

impl AssistantMessage {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            tenant_id: String::new(),
            session_id: Uuid::nil(),
            run_id: Uuid::nil(),
            content: None,
            name: None,
            tool_calls: None,
            created_at: Utc::now(),
        }
    }

    pub fn id(&self) -> &Uuid {
        &self.id
    }
    pub fn role(&self) -> &MessageRole {
        &MessageRole::Assistant
    }

    pub fn has_tool_calls(&self) -> bool {
        match &self.tool_calls {
            Some(calls) => !calls.is_empty(),
            None => false,
        }
    }

    pub fn with_id(mut self, id: impl Into<Uuid>) -> Self {
        self.id = id.into();
        self
    }

    pub fn with_tenant_id(mut self, tenant_id: impl Into<String>) -> Self {
        self.tenant_id = tenant_id.into();
        self
    }

    pub fn with_session_id(mut self, session_id: impl Into<Uuid>) -> Self {
        self.session_id = session_id.into();
        self
    }

    pub fn with_run_id(mut self, run_id: impl Into<Uuid>) -> Self {
        self.run_id = run_id.into();
        self
    }

    pub fn with_content(mut self, content: impl Into<String>) -> Self {
        self.content = Some(content.into());
        self
    }

    pub fn maybe_with_content(mut self, content: Option<impl Into<String>>) -> Self {
        if let Some(content) = content {
            self.content = Some(content.into());
        }
        self
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_tool_calls(mut self, tool_calls: Vec<ToolCall>) -> Self {
        self.tool_calls = Some(tool_calls);
        self
    }

    pub fn with_created_at(mut self, created_at: impl Into<DateTime<Utc>>) -> Self {
        self.created_at = created_at.into();
        self
    }
}

impl Default for AssistantMessage {
    fn default() -> Self {
        Self::new()
    }
}

impl ToModelType for AssistantMessage {
    type ModelType = ModelAssistantMessage;

    fn to_model_type(&self) -> Self::ModelType {
        let mut content = self
            .content
            .as_deref()
            .filter(|content| !content.is_empty())
            .map(|text| vec![ModelAssistantContent::Text(text.to_string())])
            .unwrap_or_default();

        if let Some(tool_calls) = &self.tool_calls {
            content.extend(
                tool_calls
                    .iter()
                    .map(|call| ModelAssistantContent::ToolCall(call.to_model_type())),
            );
        }

        ModelAssistantMessage { id: Some(self.id.to_string()), content }
    }
}

impl FromModelType for AssistantMessage {
    type ModelType = ModelAssistantMessage;
    type Output = (Self, Option<ReasoningMessage>);

    fn from_model_type(model: Self::ModelType) -> Self::Output {
        let mut text_parts = Vec::new();
        let mut tool_calls = Vec::new();
        let mut reasoning = None;

        for block in model.content {
            match block {
                ModelAssistantContent::Text(text) => text_parts.push(text),
                ModelAssistantContent::ToolCall(tool_call) => {
                    tool_calls.push(ToolCall::from_model_type(tool_call))
                }
                ModelAssistantContent::Reasoning(reasoning_content) => {
                    let mut visible = String::new();
                    let mut signature = None;

                    for content in reasoning_content.content {
                        match content {
                            ModelReasoningContent::Text { text, signature: sig } => {
                                visible.push_str(&text);
                                if sig.is_some() {
                                    signature = sig;
                                }
                            }
                            ModelReasoningContent::Summary(s) => visible.push_str(&s),
                            ModelReasoningContent::Encrypted(e) => signature = Some(e),
                            ModelReasoningContent::Redacted(r) => signature = Some(r),
                        }
                    }

                    let message = ReasoningMessage::new(visible);

                    match signature {
                        Some(sig) => reasoning = Some(message.with_signature(sig)),
                        None => reasoning = Some(message),
                    }
                }
                _ => {}
            }
        }

        let mut assistant = AssistantMessage::new();

        if !text_parts.is_empty() {
            assistant = assistant.with_content(text_parts.join("\n"));
        }

        if !tool_calls.is_empty() {
            assistant = assistant.with_tool_calls(tool_calls);
        }

        (assistant, reasoning)
    }
}

/// A tool message.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolMessage {
    pub id: Uuid,
    pub tenant_id: String,
    pub session_id: Uuid,
    pub run_id: Option<Uuid>,
    pub content: Option<String>,
    pub name: Option<String>,
    pub tool_call_id: String,
    pub parent_message_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl ToolMessage {
    pub fn new(tool_call_id: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            tenant_id: String::new(),
            session_id: Uuid::nil(),
            run_id: None,
            content: None,
            name: None,
            tool_call_id: tool_call_id.into(),
            parent_message_id: None,
            error: None,
            created_at: Utc::now(),
        }
    }

    pub fn id(&self) -> &Uuid {
        &self.id
    }
    pub fn role(&self) -> &MessageRole {
        &MessageRole::Tool
    }

    pub fn is_error(&self) -> bool {
        self.error.is_some()
    }

    pub fn with_id(mut self, id: impl Into<Uuid>) -> Self {
        self.id = id.into();
        self
    }

    pub fn with_tenant_id(mut self, tenant_id: impl Into<String>) -> Self {
        self.tenant_id = tenant_id.into();
        self
    }

    pub fn with_session_id(mut self, session_id: impl Into<Uuid>) -> Self {
        self.session_id = session_id.into();
        self
    }

    pub fn with_run_id(mut self, run_id: impl Into<Uuid>) -> Self {
        self.run_id = Some(run_id.into());
        self
    }

    pub fn with_content(mut self, content: impl Into<String>) -> Self {
        self.content = Some(content.into());
        self
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_parent_message_id(mut self, parent_message_id: impl Into<Uuid>) -> Self {
        self.parent_message_id = Some(parent_message_id.into());
        self
    }

    pub fn with_error(mut self, error: impl Into<String>) -> Self {
        self.error = Some(error.into());
        self
    }

    pub fn with_created_at(mut self, created_at: impl Into<DateTime<Utc>>) -> Self {
        self.created_at = created_at.into();
        self
    }
}

impl ToModelType for ToolMessage {
    type ModelType = ModelUserMessage;

    fn to_model_type(&self) -> Self::ModelType {
        ModelUserMessage {
            content: vec![ModelUserContent::ToolResult(ModelToolResult {
                call_id: self.tool_call_id.clone(),
                content: vec![ModelToolResultContent::Text(match &self.error {
                    Some(error) => format!("Error: {}", error),
                    None => self.content.clone().unwrap_or_default(),
                })],
            })],
        }
    }
}

/// A reasoning message.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReasoningMessage {
    pub id: Uuid,
    pub tenant_id: String,
    pub session_id: Uuid,
    pub run_id: Uuid,
    pub content: String,
    /// Provider-issued opaque value needed for multi-turn continuity.
    /// For text thinking blocks this is the provider's signature; for
    /// encrypted/redacted blocks this is the opaque blob itself.
    pub signature: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl ReasoningMessage {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            tenant_id: String::new(),
            session_id: Uuid::nil(),
            run_id: Uuid::nil(),
            content: content.into(),
            signature: None,
            created_at: Utc::now(),
        }
    }

    pub fn id(&self) -> &Uuid {
        &self.id
    }
    pub fn role(&self) -> &MessageRole {
        &MessageRole::Reasoning
    }

    pub fn with_id(mut self, id: impl Into<Uuid>) -> Self {
        self.id = id.into();
        self
    }

    pub fn with_tenant_id(mut self, tenant_id: impl Into<String>) -> Self {
        self.tenant_id = tenant_id.into();
        self
    }

    pub fn with_session_id(mut self, session_id: impl Into<Uuid>) -> Self {
        self.session_id = session_id.into();
        self
    }

    pub fn with_run_id(mut self, run_id: impl Into<Uuid>) -> Self {
        self.run_id = run_id.into();
        self
    }

    pub fn with_signature(mut self, signature: impl Into<String>) -> Self {
        self.signature = Some(signature.into());
        self
    }

    pub fn with_created_at(mut self, created_at: impl Into<DateTime<Utc>>) -> Self {
        self.created_at = created_at.into();
        self
    }
}

impl ToModelType for ReasoningMessage {
    type ModelType = ModelAssistantMessage;

    fn to_model_type(&self) -> Self::ModelType {
        ModelAssistantMessage {
            id: Some(self.id.to_string()),
            content: vec![ModelAssistantContent::Reasoning(ModelReasoning {
                id: Some(self.id.to_string()),
                content: if self.content.is_empty() {
                    // Encrypted/redacted block, no visible text, pass opaque blob as-is.
                    match &self.signature {
                        Some(sig) => vec![ModelReasoningContent::Encrypted(sig.clone())],
                        None => vec![],
                    }
                } else {
                    // Text thinking block, include the provider signature
                    vec![ModelReasoningContent::Text {
                        text: self.content.clone(),
                        signature: self.signature.clone(),
                    }]
                },
            })],
        }
    }
}

/// An enum representing different types of messages exchanged between the agent, the user, other agents, and tools.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Message {
    /// A system message.
    System(SystemMessage),
    /// A user message.
    User(UserMessage),
    /// An assistant message, which may include tool calls.
    Assistant(AssistantMessage),
    /// A tool message, which may include an error if the tool execution failed.
    Tool(ToolMessage),
    /// A reasoning message.
    Reasoning(ReasoningMessage),
}

impl Message {
    /// Returns the ID of the message.
    pub fn id(&self) -> &Uuid {
        match self {
            Message::System(m) => m.id(),
            Message::User(m) => m.id(),
            Message::Assistant(m) => m.id(),
            Message::Tool(m) => m.id(),
            Message::Reasoning(m) => m.id(),
        }
    }

    /// Returns the tenant ID of the message.
    pub fn tenant_id(&self) -> &str {
        match self {
            Message::System(m) => &m.tenant_id,
            Message::User(m) => &m.tenant_id,
            Message::Assistant(m) => &m.tenant_id,
            Message::Tool(m) => &m.tenant_id,
            Message::Reasoning(m) => &m.tenant_id,
        }
    }

    /// Returns the session ID of the message.
    pub fn session_id(&self) -> &Uuid {
        match self {
            Message::System(m) => &m.session_id,
            Message::User(m) => &m.session_id,
            Message::Assistant(m) => &m.session_id,
            Message::Tool(m) => &m.session_id,
            Message::Reasoning(m) => &m.session_id,
        }
    }

    /// Returns the run ID of the message.
    pub fn run_id(&self) -> Option<&Uuid> {
        match self {
            Message::System(m) => m.run_id.as_ref(),
            Message::User(m) => m.run_id.as_ref(),
            Message::Assistant(m) => Some(&m.run_id),
            Message::Tool(m) => m.run_id.as_ref(),
            Message::Reasoning(m) => Some(&m.run_id),
        }
    }

    /// Returns the role of the message.
    pub fn role(&self) -> &MessageRole {
        match self {
            Message::System(m) => m.role(),
            Message::User(m) => m.role(),
            Message::Assistant(m) => m.role(),
            Message::Tool(m) => m.role(),
            Message::Reasoning(m) => m.role(),
        }
    }

    /// Returns the creation timestamp of the message.
    pub fn created_at(&self) -> &DateTime<Utc> {
        match self {
            Message::System(m) => &m.created_at,
            Message::User(m) => &m.created_at,
            Message::Assistant(m) => &m.created_at,
            Message::Tool(m) => &m.created_at,
            Message::Reasoning(m) => &m.created_at,
        }
    }

    /// Returns the content of the message, if applicable.
    pub fn content(&self) -> Option<&str> {
        match self {
            Message::System(m) => Some(&m.content),
            Message::User(m) => Some(&m.content),
            Message::Assistant(m) => m.content.as_deref(),
            Message::Tool(m) => m.content.as_deref(),
            Message::Reasoning(m) => Some(&m.content),
        }
    }

    /// Create a new system message with the given content.
    pub fn system(content: impl Into<String>) -> Self {
        Message::System(SystemMessage::new(content))
    }

    /// Try to unwrap the message as a system message.
    pub fn as_system(&self) -> Option<&SystemMessage> {
        match self {
            Message::System(m) => Some(m),
            _ => None,
        }
    }

    /// Try to unwrap the message as a mutable system message.
    pub fn as_system_mut(&mut self) -> Option<&mut SystemMessage> {
        match self {
            Message::System(m) => Some(m),
            _ => None,
        }
    }

    /// Create a new user message with the given content.
    pub fn user(content: impl Into<String>) -> Self {
        Message::User(UserMessage::new(content))
    }

    /// Try to unwrap the message as a user message.
    pub fn as_user(&self) -> Option<&UserMessage> {
        match self {
            Message::User(m) => Some(m),
            _ => None,
        }
    }

    /// Try to unwrap the message as a mutable user message.
    pub fn as_user_mut(&mut self) -> Option<&mut UserMessage> {
        match self {
            Message::User(m) => Some(m),
            _ => None,
        }
    }

    /// Create a new assistant message with the given content.
    pub fn assistant(content: impl Into<String>) -> Self {
        Message::Assistant(AssistantMessage::new().with_content(content))
    }

    /// Try to unwrap the message as an assistant message.
    pub fn as_assistant(&self) -> Option<&AssistantMessage> {
        match self {
            Message::Assistant(m) => Some(m),
            _ => None,
        }
    }

    /// Try to unwrap the message as a mutable assistant message.
    pub fn as_assistant_mut(&mut self) -> Option<&mut AssistantMessage> {
        match self {
            Message::Assistant(m) => Some(m),
            _ => None,
        }
    }

    /// Create a new tool message with the given content and tool call ID.
    pub fn tool(
        tool_call_id: impl Into<String>,
        name: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        Message::Tool(
            ToolMessage::new(tool_call_id)
                .with_name(name)
                .with_content(content),
        )
    }

    /// Create a new tool error message with the given error content and tool call ID.
    pub fn tool_error(
        tool_call_id: impl Into<String>,
        name: impl Into<String>,
        error: impl Into<String>,
    ) -> Self {
        Message::Tool(
            ToolMessage::new(tool_call_id)
                .with_name(name)
                .with_error(error),
        )
    }

    /// Try to unwrap the message as a tool message.
    pub fn as_tool(&self) -> Option<&ToolMessage> {
        match self {
            Message::Tool(m) => Some(m),
            _ => None,
        }
    }

    /// Try to unwrap the message as a mutable tool message.
    pub fn as_tool_mut(&mut self) -> Option<&mut ToolMessage> {
        match self {
            Message::Tool(m) => Some(m),
            _ => None,
        }
    }

    /// Create a new reasoning message with the given content.
    pub fn reasoning(content: impl Into<String>) -> Self {
        Message::Reasoning(ReasoningMessage::new(content))
    }

    /// Try to unwrap the message as a reasoning message.
    pub fn as_reasoning(&self) -> Option<&ReasoningMessage> {
        match self {
            Message::Reasoning(m) => Some(m),
            _ => None,
        }
    }

    /// Try to unwrap the message as a mutable reasoning message.
    pub fn as_reasoning_mut(&mut self) -> Option<&mut ReasoningMessage> {
        match self {
            Message::Reasoning(m) => Some(m),
            _ => None,
        }
    }

    /// Set the tenant ID for the message.
    pub fn with_tenant_id(self, tenant_id: impl Into<String>) -> Self {
        match self {
            Message::System(message) => Message::System(message.with_tenant_id(tenant_id)),
            Message::User(message) => Message::User(message.with_tenant_id(tenant_id)),
            Message::Assistant(message) => Message::Assistant(message.with_tenant_id(tenant_id)),
            Message::Tool(message) => Message::Tool(message.with_tenant_id(tenant_id)),
            Message::Reasoning(message) => Message::Reasoning(message.with_tenant_id(tenant_id)),
        }
    }

    /// Optionally set the tenant ID for the message if the input is Some.
    pub fn maybe_with_tenant_id(self, tenant_id: Option<impl Into<String>>) -> Self {
        match tenant_id {
            Some(tenant_id) => self.with_tenant_id(tenant_id),
            None => self,
        }
    }

    /// Set the session ID for the message.
    pub fn with_session_id(self, session_id: impl Into<Uuid>) -> Self {
        match self {
            Message::System(message) => Message::System(message.with_session_id(session_id)),
            Message::User(message) => Message::User(message.with_session_id(session_id)),
            Message::Assistant(message) => Message::Assistant(message.with_session_id(session_id)),
            Message::Tool(message) => Message::Tool(message.with_session_id(session_id)),
            Message::Reasoning(message) => Message::Reasoning(message.with_session_id(session_id)),
        }
    }

    /// Optionally set the session ID for the message if the input is Some.
    pub fn maybe_with_session_id(self, session_id: Option<impl Into<Uuid>>) -> Self {
        match session_id {
            Some(session_id) => self.with_session_id(session_id),
            None => self,
        }
    }

    /// Set the run ID for the message.
    pub fn with_run_id(self, run_id: impl Into<Uuid>) -> Self {
        match self {
            Message::System(message) => Message::System(message.with_run_id(run_id)),
            Message::User(message) => Message::User(message.with_run_id(run_id)),
            Message::Assistant(message) => Message::Assistant(message.with_run_id(run_id)),
            Message::Tool(message) => Message::Tool(message.with_run_id(run_id)),
            Message::Reasoning(message) => Message::Reasoning(message.with_run_id(run_id)),
        }
    }

    /// Optionally set the run ID for the message if the input is Some.
    pub fn maybe_with_run_id(self, run_id: Option<impl Into<Uuid>>) -> Self {
        match run_id {
            Some(run_id) => self.with_run_id(run_id),
            None => self,
        }
    }

    /// Set the information from the context.
    pub fn with_context<E, M>(self, ctx: &AgentContext<E, M>) -> Self
    where
        E: Send + Clone + 'static,
        M: Send + Clone + 'static,
    {
        match self {
            Message::System(message) => Message::System(
                message
                    .with_tenant_id(ctx.tenant_id.clone())
                    .with_session_id(ctx.session_id)
                    .with_run_id(ctx.run_id),
            ),
            Message::User(message) => Message::User(
                message
                    .with_tenant_id(ctx.tenant_id.clone())
                    .with_session_id(ctx.session_id)
                    .with_run_id(ctx.run_id),
            ),
            Message::Assistant(message) => Message::Assistant(
                message
                    .with_tenant_id(ctx.tenant_id.clone())
                    .with_session_id(ctx.session_id)
                    .with_run_id(ctx.run_id),
            ),
            Message::Tool(message) => Message::Tool(
                message
                    .with_tenant_id(ctx.tenant_id.clone())
                    .with_session_id(ctx.session_id)
                    .with_run_id(ctx.run_id),
            ),
            Message::Reasoning(message) => Message::Reasoning(
                message
                    .with_tenant_id(ctx.tenant_id.clone())
                    .with_session_id(ctx.session_id)
                    .with_run_id(ctx.run_id),
            ),
        }
    }
}

impl TokenCount for Message {
    fn message_content(&self) -> Option<&str> {
        self.content()
    }
}

impl MessageGroup for Message {
    fn group_id(&self) -> Option<String> {
        match self {
            Message::Assistant(message) if message.has_tool_calls() => {
                Some(message.id().to_string())
            }
            Message::Tool(message) => message
                .parent_message_id
                .as_ref()
                .map(|id| id.to_string()),
            _ => None,
        }
    }
}

impl ToModelType for Message {
    type ModelType = ModelChatMessage;

    fn to_model_type(&self) -> Self::ModelType {
        match self {
            Message::System(m) => ModelChatMessage::System(m.to_model_type()),
            Message::User(m) => ModelChatMessage::User(m.to_model_type()),
            Message::Assistant(m) => ModelChatMessage::Assistant(m.to_model_type()),
            Message::Tool(m) => ModelChatMessage::User(m.to_model_type()),
            Message::Reasoning(m) => ModelChatMessage::Assistant(m.to_model_type()),
        }
    }
}

/// An ordered sequence of messages that can be converted to model messages as a unit.
///
/// Unlike converting messages individually, this type is aware of adjacent messages
/// and can merge pairs that must be represented as a single model turn. For example,
/// a `Reasoning` followed by an `Assistant` must become one assistant message whose
/// content array contains the reasoning block(s) before the text or tool-call blocks.
pub struct MessageList(Vec<Message>);

impl MessageList {
    pub fn new(messages: Vec<Message>) -> Self {
        Self(messages)
    }

    /// Returns a slice of the messages in the list.
    pub fn messages(&self) -> &[Message] {
        &self.0
    }

    /// Returns a mutable slice of the messages in the list.
    pub fn messages_mut(&mut self) -> &mut [Message] {
        &mut self.0
    }

    /// Consumes the list and returns the inner vector of messages.
    pub fn into_messages(self) -> Vec<Message> {
        self.0
    }
}

impl FromIterator<Message> for MessageList {
    fn from_iter<I: IntoIterator<Item = Message>>(iter: I) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl ToModelType for MessageList {
    type ModelType = Vec<ModelChatMessage>;

    fn to_model_type(&self) -> Self::ModelType {
        let mut result = Vec::new();
        let mut idx = 0;
        let messages = self.messages();

        while idx < messages.len() {
            match (&messages[idx], messages.get(idx + 1)) {
                (Message::Reasoning(reasoning), Some(Message::Assistant(assistant))) => {
                    // Merge reasoning and assistant into a single turn: reasoning content first,
                    // followed by the assistant's text/tool-call content.
                    let reasoning = reasoning.to_model_type();
                    let assistant = assistant.to_model_type();

                    result.push(ModelChatMessage::Assistant(ModelAssistantMessage {
                        id: assistant.id,
                        content: reasoning
                            .content
                            .into_iter()
                            .chain(assistant.content)
                            .collect(),
                    }));
                    idx += 2;
                }
                (Message::Tool(_), _) => {
                    // Collect this and all immediately following Tool messages
                    // into one User turn with multiple ToolResult content blocks.
                    let mut content = Vec::new();

                    while let Some(Message::Tool(tool_message)) = messages.get(idx) {
                        content.extend(tool_message.to_model_type().content);
                        idx += 1;
                    }

                    result.push(ModelChatMessage::User(ModelUserMessage { content }));
                }
                _ => {
                    result.push(messages[idx].to_model_type());
                    idx += 1;
                }
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agentc_agent::types::tools::ToolCall;
    use agentc_model::types::message::ChatMessage as ModelChatMessage;
    use agentc_model::types::message::UserContent as ModelUserContent;
    use serde_json::Value;

    fn assistant_with_tool_calls(call_ids: &[&str]) -> Message {
        Message::Assistant(
            AssistantMessage::new().with_tool_calls(
                call_ids
                    .iter()
                    .map(|id| ToolCall {
                        id: id.to_string(),
                        name: "some_tool".to_string(),
                        arguments: Value::Object(Default::default()),
                    })
                    .collect(),
            ),
        )
    }

    fn tool_result(call_id: &str, content: &str) -> Message {
        Message::Tool(ToolMessage::new(call_id).with_content(content))
    }

    #[test]
    fn two_tool_messages_merged_into_single_user_turn() {
        let list = MessageList::new(vec![
            assistant_with_tool_calls(&["call-1", "call-2"]),
            tool_result("call-1", "result-a"),
            tool_result("call-2", "result-b"),
        ]);
        let model = list.to_model_type();
        // Expect [Assistant, User]
        // 2 items, not 3.
        assert_eq!(model.len(), 2);
        let ModelChatMessage::User(user) = &model[1] else {
            panic!("expected User message at index 1");
        };
        // The single user turn must carry both tool results.
        assert_eq!(user.content.len(), 2);
        let call_ids: Vec<&str> = user
            .content
            .iter()
            .filter_map(|c| {
                if let ModelUserContent::ToolResult(r) = c {
                    Some(r.call_id.as_str())
                } else {
                    None
                }
            })
            .collect();
        assert!(call_ids.contains(&"call-1"));
        assert!(call_ids.contains(&"call-2"));
    }

    #[test]
    fn single_tool_message_produces_single_user_turn() {
        let list = MessageList::new(vec![
            assistant_with_tool_calls(&["call-1"]),
            tool_result("call-1", "result"),
        ]);
        let model = list.to_model_type();
        assert_eq!(model.len(), 2);
        let ModelChatMessage::User(user) = &model[1] else {
            panic!("expected User at index 1");
        };
        assert_eq!(user.content.len(), 1);
    }

    #[test]
    fn reasoning_and_assistant_merged_into_single_assistant_turn() {
        let list = MessageList::new(vec![
            Message::Reasoning(ReasoningMessage::new("thinking...")),
            Message::Assistant(AssistantMessage::new().with_content("reply")),
        ]);
        let model = list.to_model_type();
        assert_eq!(model.len(), 1);
        assert!(matches!(model[0], ModelChatMessage::Assistant(_)));
    }

    #[test]
    fn tool_messages_not_merged_across_assistant_boundary() {
        let list = MessageList::new(vec![
            assistant_with_tool_calls(&["a"]),
            tool_result("a", "result-a"),
            assistant_with_tool_calls(&["b", "c"]),
            tool_result("b", "result-b"),
            tool_result("c", "result-c"),
        ]);
        let model = list.to_model_type();
        // [Asst, User(1), Asst, User(2)] = 4 items
        assert_eq!(model.len(), 4);
        let ModelChatMessage::User(user1) = &model[1] else {
            panic!("expected User at 1");
        };
        assert_eq!(user1.content.len(), 1);
        let ModelChatMessage::User(user2) = &model[3] else {
            panic!("expected User at 3");
        };
        assert_eq!(user2.content.len(), 2);
    }
}
