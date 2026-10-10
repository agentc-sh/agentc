// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use agentc_agent::types::conversion::ToModelType;
use agentc_model::types::{
    message::{UserContent as ModelUserContent, UserMessage as ModelUserMessage},
    tools::{ToolResult as ModelToolResult, ToolResultContent as ModelToolResultContent},
};

use super::MessageRole;

/// A tool message.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolMessage {
    pub id: Uuid,
    pub tenant_id: String,
    pub session_id: Uuid,
    pub run_id: Option<Uuid>,
    pub checkpoint_id: Option<Uuid>,
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
            checkpoint_id: None,
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

    pub fn with_checkpoint_id(mut self, checkpoint_id: impl Into<Uuid>) -> Self {
        self.checkpoint_id = Some(checkpoint_id.into());
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
