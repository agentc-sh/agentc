// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use agentc_agent::types::conversion::{FromModelType, ToModelType};
use agentc_model::types::message::SystemMessage as ModelSystemMessage;

use super::MessageRole;

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
