// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use agentc_agent::types::conversion::ToModelType;
use agentc_model::types::{
    message::{
        AssistantContent as ModelAssistantContent, AssistantMessage as ModelAssistantMessage,
    },
    reasoning::{Reasoning as ModelReasoning, ReasoningContent as ModelReasoningContent},
};

use super::MessageRole;

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
