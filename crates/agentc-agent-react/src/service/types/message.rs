// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use agentc_agent::types::tools::ToolCall;

use crate::{
    repository::message::params::FindMessageParams as RepoFindMessageParams,
    types::message::{
        AssistantMessage as DomainAssistantMessage, Message as DomainMessage, MessageRole,
        ReasoningMessage as DomainReasoningMessage, SystemMessage as DomainSystemMessage,
        ToolMessage as DomainToolMessage, UserContent, UserMessage as DomainUserMessage,
    },
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSystemMessageParams {
    pub id: Uuid,
    pub content: String,
    pub name: Option<String>,
}

impl CreateSystemMessageParams {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            content: content.into(),
            name: None,
        }
    }

    pub fn with_id(mut self, id: impl Into<Uuid>) -> Self {
        self.id = id.into();
        self
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn to_entity(
        &self,
        tenant_id: impl Into<String>,
        session_id: impl Into<Uuid>,
    ) -> DomainSystemMessage {
        DomainSystemMessage {
            id: self.id,
            tenant_id: tenant_id.into(),
            session_id: session_id.into(),
            run_id: None,
            checkpoint_id: None,
            content: self.content.clone(),
            name: self.name.clone(),
            created_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMessageResponse {
    pub id: Uuid,
    pub tenant_id: String,
    pub session_id: Uuid,
    pub run_id: Option<Uuid>,
    pub checkpoint_id: Option<Uuid>,
    pub content: String,
    pub name: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl SystemMessageResponse {
    pub fn from_entity(entity: &DomainSystemMessage) -> Self {
        Self {
            id: entity.id,
            tenant_id: entity.tenant_id.clone(),
            session_id: entity.session_id,
            run_id: entity.run_id,
            checkpoint_id: entity.checkpoint_id,
            content: entity.content.clone(),
            name: entity.name.clone(),
            created_at: entity.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserMessageParams {
    pub id: Uuid,
    pub content: Vec<UserContent>,
    pub name: Option<String>,
}

impl CreateUserMessageParams {
    pub fn new(content: impl Into<String>) -> Self {
        Self::from_content([UserContent::text(content)])
    }

    pub fn from_content(content: impl IntoIterator<Item = impl Into<UserContent>>) -> Self {
        Self {
            id: Uuid::new_v4(),
            content: content
                .into_iter()
                .map(Into::into)
                .collect(),
            name: None,
        }
    }

    pub fn with_id(mut self, id: impl Into<Uuid>) -> Self {
        self.id = id.into();
        self
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_content(
        mut self,
        content: impl IntoIterator<Item = impl Into<UserContent>>,
    ) -> Self {
        self.content = content
            .into_iter()
            .map(Into::into)
            .collect();
        self
    }

    pub fn to_entity(
        &self,
        tenant_id: impl Into<String>,
        session_id: impl Into<Uuid>,
    ) -> DomainUserMessage {
        DomainUserMessage {
            id: self.id,
            tenant_id: tenant_id.into(),
            session_id: session_id.into(),
            run_id: None,
            checkpoint_id: None,
            content: self.content.clone(),
            name: self.name.clone(),
            created_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMessageResponse {
    pub id: Uuid,
    pub tenant_id: String,
    pub session_id: Uuid,
    pub run_id: Option<Uuid>,
    pub checkpoint_id: Option<Uuid>,
    pub content: Vec<UserContent>,
    pub name: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl UserMessageResponse {
    pub fn from_entity(entity: &DomainUserMessage) -> Self {
        Self {
            id: entity.id,
            tenant_id: entity.tenant_id.clone(),
            session_id: entity.session_id,
            run_id: entity.run_id,
            checkpoint_id: entity.checkpoint_id,
            content: entity.content.clone(),
            name: entity.name.clone(),
            created_at: entity.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantMessageResponse {
    pub id: Uuid,
    pub tenant_id: String,
    pub session_id: Uuid,
    pub run_id: Uuid,
    pub checkpoint_id: Option<Uuid>,
    pub content: Option<String>,
    pub name: Option<String>,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub created_at: DateTime<Utc>,
}

impl AssistantMessageResponse {
    pub fn from_entity(entity: &DomainAssistantMessage) -> Self {
        Self {
            id: entity.id,
            tenant_id: entity.tenant_id.clone(),
            session_id: entity.session_id,
            run_id: entity.run_id,
            checkpoint_id: entity.checkpoint_id,
            content: entity.content.clone(),
            name: entity.name.clone(),
            tool_calls: entity.tool_calls.clone(),
            created_at: entity.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateToolMessageParams {
    pub id: Uuid,
    pub tool_call_id: String,
    pub parent_message_id: Option<Uuid>,
    pub content: Option<String>,
    pub name: Option<String>,
    pub error: Option<String>,
}

impl CreateToolMessageParams {
    pub fn new(tool_call_id: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            tool_call_id: tool_call_id.into(),
            parent_message_id: None,
            content: None,
            name: None,
            error: None,
        }
    }

    pub fn with_id(mut self, id: impl Into<Uuid>) -> Self {
        self.id = id.into();
        self
    }

    pub fn with_parent_message_id(mut self, parent_message_id: impl Into<Uuid>) -> Self {
        self.parent_message_id = Some(parent_message_id.into());
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

    pub fn with_error(mut self, error: impl Into<String>) -> Self {
        self.error = Some(error.into());
        self
    }

    pub fn to_entity(
        &self,
        tenant_id: impl Into<String>,
        session_id: impl Into<Uuid>,
    ) -> DomainToolMessage {
        DomainToolMessage {
            id: self.id,
            tenant_id: tenant_id.into(),
            session_id: session_id.into(),
            run_id: None,
            checkpoint_id: None,
            content: self.content.clone(),
            name: self.name.clone(),
            tool_call_id: self.tool_call_id.clone(),
            parent_message_id: self.parent_message_id,
            error: self.error.clone(),
            created_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMessageResponse {
    pub id: Uuid,
    pub tenant_id: String,
    pub session_id: Uuid,
    pub run_id: Option<Uuid>,
    pub checkpoint_id: Option<Uuid>,
    pub content: Option<String>,
    pub name: Option<String>,
    pub tool_call_id: String,
    pub parent_message_id: Option<Uuid>,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl ToolMessageResponse {
    pub fn from_entity(entity: &DomainToolMessage) -> Self {
        Self {
            id: entity.id,
            tenant_id: entity.tenant_id.clone(),
            session_id: entity.session_id,
            run_id: entity.run_id,
            checkpoint_id: entity.checkpoint_id,
            content: entity.content.clone(),
            name: entity.name.clone(),
            tool_call_id: entity.tool_call_id.clone(),
            parent_message_id: entity.parent_message_id,
            error: entity.error.clone(),
            created_at: entity.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningMessageResponse {
    pub id: Uuid,
    pub tenant_id: String,
    pub session_id: Uuid,
    pub run_id: Uuid,
    pub checkpoint_id: Option<Uuid>,
    pub content: String,
    pub signature: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl ReasoningMessageResponse {
    pub fn from_entity(entity: &DomainReasoningMessage) -> Self {
        Self {
            id: entity.id,
            tenant_id: entity.tenant_id.clone(),
            session_id: entity.session_id,
            run_id: entity.run_id,
            checkpoint_id: entity.checkpoint_id,
            content: entity.content.clone(),
            signature: entity.signature.clone(),
            created_at: entity.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "role", rename_all = "snake_case")]
pub enum CreateMessageParams {
    System(CreateSystemMessageParams),
    User(CreateUserMessageParams),
    Tool(CreateToolMessageParams),
}

impl CreateMessageParams {
    pub fn id(&self) -> &Uuid {
        match self {
            CreateMessageParams::System(params) => &params.id,
            CreateMessageParams::User(params) => &params.id,
            CreateMessageParams::Tool(params) => &params.id,
        }
    }

    pub fn to_entity(
        &self,
        tenant_id: impl Into<String>,
        session_id: impl Into<Uuid>,
    ) -> DomainMessage {
        match self {
            CreateMessageParams::System(params) => {
                DomainMessage::System(params.to_entity(tenant_id, session_id))
            }
            CreateMessageParams::User(params) => {
                DomainMessage::User(params.to_entity(tenant_id, session_id))
            }
            CreateMessageParams::Tool(params) => {
                DomainMessage::Tool(params.to_entity(tenant_id, session_id))
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "role", rename_all = "snake_case")]
pub enum MessageResponse {
    System(SystemMessageResponse),
    User(UserMessageResponse),
    Assistant(AssistantMessageResponse),
    Tool(ToolMessageResponse),
    Reasoning(ReasoningMessageResponse),
}

impl MessageResponse {
    pub fn from_entity(entity: &DomainMessage) -> Self {
        match entity {
            DomainMessage::System(msg) => {
                MessageResponse::System(SystemMessageResponse::from_entity(msg))
            }
            DomainMessage::User(msg) => {
                MessageResponse::User(UserMessageResponse::from_entity(msg))
            }
            DomainMessage::Assistant(msg) => {
                MessageResponse::Assistant(AssistantMessageResponse::from_entity(msg))
            }
            DomainMessage::Tool(msg) => {
                MessageResponse::Tool(ToolMessageResponse::from_entity(msg))
            }
            DomainMessage::Reasoning(msg) => {
                MessageResponse::Reasoning(ReasoningMessageResponse::from_entity(msg))
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindMessageParams {
    pub per_page: Option<u64>,
    pub page: Option<String>,
    pub tenant_ids: Option<Vec<String>>,
    pub ids: Option<Vec<Uuid>>,
    pub session_ids: Option<Vec<Uuid>>,
    pub run_ids: Option<Vec<Uuid>>,
    pub checkpoint_ids: Option<Vec<Uuid>>,
    pub roles: Option<Vec<MessageRole>>,
    pub created_before: Option<DateTime<Utc>>,
    pub created_after: Option<DateTime<Utc>>,
}

impl FindMessageParams {
    pub fn new() -> Self {
        Self {
            per_page: Some(10),
            page: None,
            tenant_ids: None,
            ids: None,
            session_ids: None,
            run_ids: None,
            checkpoint_ids: None,
            roles: None,
            created_before: None,
            created_after: None,
        }
    }

    pub fn per_page(mut self, per_page: impl Into<u64>) -> Self {
        self.per_page = Some(per_page.into());
        self
    }

    pub fn maybe_per_page(mut self, per_page: Option<impl Into<u64>>) -> Self {
        self.per_page = per_page.map(Into::into);
        self
    }

    pub fn no_limit(mut self) -> Self {
        self.per_page = None;
        self
    }

    pub fn page(mut self, page: impl Into<String>) -> Self {
        self.page = Some(page.into());
        self
    }

    pub fn tenant_ids(mut self, ids: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.tenant_ids = Some(
            ids.into_iter()
                .map(Into::into)
                .collect(),
        );
        self
    }

    pub fn ids(mut self, ids: impl IntoIterator<Item = impl Into<Uuid>>) -> Self {
        self.ids = Some(
            ids.into_iter()
                .map(Into::into)
                .collect(),
        );
        self
    }

    pub fn session_ids(mut self, ids: impl IntoIterator<Item = impl Into<Uuid>>) -> Self {
        self.session_ids = Some(
            ids.into_iter()
                .map(Into::into)
                .collect(),
        );
        self
    }

    pub fn run_ids(mut self, ids: impl IntoIterator<Item = impl Into<Uuid>>) -> Self {
        self.run_ids = Some(
            ids.into_iter()
                .map(Into::into)
                .collect(),
        );
        self
    }

    pub fn checkpoint_ids(mut self, ids: impl IntoIterator<Item = impl Into<Uuid>>) -> Self {
        self.checkpoint_ids = Some(
            ids.into_iter()
                .map(Into::into)
                .collect(),
        );
        self
    }

    pub fn roles(mut self, roles: impl IntoIterator<Item = impl Into<MessageRole>>) -> Self {
        self.roles = Some(
            roles
                .into_iter()
                .map(Into::into)
                .collect(),
        );
        self
    }

    pub fn created_before(mut self, created_before: impl Into<DateTime<Utc>>) -> Self {
        self.created_before = Some(created_before.into());
        self
    }

    pub fn created_after(mut self, created_after: impl Into<DateTime<Utc>>) -> Self {
        self.created_after = Some(created_after.into());
        self
    }
}

impl Default for FindMessageParams {
    fn default() -> Self {
        Self::new()
    }
}

impl From<FindMessageParams> for RepoFindMessageParams {
    fn from(params: FindMessageParams) -> Self {
        Self {
            per_page: params.per_page,
            page: params.page,
            tenant_ids: params.tenant_ids,
            ids: params.ids,
            session_ids: params.session_ids,
            run_ids: params.run_ids,
            checkpoint_ids: params.checkpoint_ids,
            roles: params.roles,
            created_before: params.created_before,
            created_after: params.created_after,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::types::message::MediaSource;

    fn user_content() -> Vec<UserContent> {
        vec![
            UserContent::text("Describe this image"),
            UserContent::image(MediaSource::Base64("image-data".to_string()), "image/png"),
        ]
    }

    #[test]
    fn user_params_preserve_content_in_entity() {
        let content = user_content();
        let entity = CreateUserMessageParams::from_content(content.clone())
            .to_entity("tenant", Uuid::new_v4());

        assert_eq!(entity.content, content);
    }

    #[test]
    fn user_response_preserves_entity_content() {
        let content = user_content();
        let entity = CreateUserMessageParams::from_content(content.clone())
            .to_entity("tenant", Uuid::new_v4());
        let response = UserMessageResponse::from_entity(&entity);

        assert_eq!(response.content, content);
    }
}
