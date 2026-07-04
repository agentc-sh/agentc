// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use agentc_agent::types::{
    conversion::{FromModelType, ToModelType},
    tools::ToolCall,
};
use agentc_model::types::{
    message::{
        AssistantContent as ModelAssistantContent, AssistantMessage as ModelAssistantMessage,
    },
    reasoning::ReasoningContent as ModelReasoningContent,
};

use super::{MessageRole, ReasoningMessage};

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
