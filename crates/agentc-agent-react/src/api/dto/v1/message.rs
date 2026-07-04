// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::{IntoParams, ToSchema};
use url::Url;
use uuid::Uuid;
use validator::{Validate, ValidateArgs, ValidationErrors};

use agentc_agent::types::tools::ToolCall;

use crate::{
    service::types::message::{
        AssistantMessageResponse, CreateMessageParams, CreateSystemMessageParams,
        CreateToolMessageParams, CreateUserMessageParams, FindMessageParams, MessageResponse,
        ReasoningMessageResponse, SystemMessageResponse, ToolMessageResponse, UserMessageResponse,
    },
    types::message::{
        Audio as DomainAudio, Document as DomainDocument, Image as DomainImage,
        MediaSource as DomainMediaSource, MessageRole, UserContent as DomainUserContent,
        Video as DomainVideo,
    },
};

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "role", rename_all = "snake_case")]
pub enum CreateMessageRequestDTO {
    System(CreateSystemMessageRequestDTO),
    User(CreateUserMessageRequestDTO),
    Tool(CreateToolMessageRequestDTO),
}

impl CreateMessageRequestDTO {
    pub fn to_params(&self) -> CreateMessageParams {
        match self {
            CreateMessageRequestDTO::System(req) => CreateMessageParams::System(req.to_params()),
            CreateMessageRequestDTO::User(req) => CreateMessageParams::User(req.to_params()),
            CreateMessageRequestDTO::Tool(req) => CreateMessageParams::Tool(req.to_params()),
        }
    }
}

impl<'v_a> ValidateArgs<'v_a> for CreateMessageRequestDTO {
    type Args = ();

    fn validate_with_args(&self, args: Self::Args) -> Result<(), ValidationErrors> {
        match self {
            CreateMessageRequestDTO::System(req) => req.validate_with_args(args),
            CreateMessageRequestDTO::User(req) => req.validate_with_args(args),
            CreateMessageRequestDTO::Tool(req) => req.validate_with_args(args),
        }
    }
}

impl Validate for CreateMessageRequestDTO {
    fn validate(&self) -> Result<(), ValidationErrors> {
        self.validate_with_args(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct CreateSystemMessageRequestDTO {
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,
    pub content: String,
}

impl CreateSystemMessageRequestDTO {
    pub fn to_params(&self) -> CreateSystemMessageParams {
        CreateSystemMessageParams {
            name: None,
            id: self.id,
            content: self.content.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum MediaSourceDTO {
    Url(Url),
    Base64(String),
}

impl MediaSourceDTO {
    pub fn to_entity(&self) -> DomainMediaSource {
        match self {
            MediaSourceDTO::Url(url) => DomainMediaSource::Url(url.clone()),
            MediaSourceDTO::Base64(data) => DomainMediaSource::Base64(data.clone()),
        }
    }

    pub fn from_entity(entity: DomainMediaSource) -> Self {
        match entity {
            DomainMediaSource::Url(url) => MediaSourceDTO::Url(url),
            DomainMediaSource::Base64(data) => MediaSourceDTO::Base64(data),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ImageDTO {
    pub source: MediaSourceDTO,
    pub media_type: String,
}

impl ImageDTO {
    pub fn to_entity(&self) -> DomainImage {
        DomainImage {
            source: self.source.to_entity(),
            media_type: self.media_type.clone(),
        }
    }

    pub fn from_entity(entity: DomainImage) -> Self {
        Self {
            source: MediaSourceDTO::from_entity(entity.source),
            media_type: entity.media_type,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AudioDTO {
    pub source: MediaSourceDTO,
    pub media_type: String,
}

impl AudioDTO {
    pub fn to_entity(&self) -> DomainAudio {
        DomainAudio {
            source: self.source.to_entity(),
            media_type: self.media_type.clone(),
        }
    }

    pub fn from_entity(entity: DomainAudio) -> Self {
        Self {
            source: MediaSourceDTO::from_entity(entity.source),
            media_type: entity.media_type,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VideoDTO {
    pub source: MediaSourceDTO,
    pub media_type: String,
}

impl VideoDTO {
    pub fn to_entity(&self) -> DomainVideo {
        DomainVideo {
            source: self.source.to_entity(),
            media_type: self.media_type.clone(),
        }
    }

    pub fn from_entity(entity: DomainVideo) -> Self {
        Self {
            source: MediaSourceDTO::from_entity(entity.source),
            media_type: entity.media_type,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DocumentDTO {
    pub source: MediaSourceDTO,
    pub media_type: String,
}

impl DocumentDTO {
    pub fn to_entity(&self) -> DomainDocument {
        DomainDocument {
            source: self.source.to_entity(),
            media_type: self.media_type.clone(),
        }
    }

    pub fn from_entity(entity: DomainDocument) -> Self {
        Self {
            source: MediaSourceDTO::from_entity(entity.source),
            media_type: entity.media_type,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum UserContentDTO {
    Text(String),
    Image(ImageDTO),
    Audio(AudioDTO),
    Video(VideoDTO),
    Document(DocumentDTO),
}

impl UserContentDTO {
    pub fn to_entity(&self) -> DomainUserContent {
        match self {
            UserContentDTO::Text(text) => DomainUserContent::Text(text.clone()),
            UserContentDTO::Image(image) => DomainUserContent::Image(image.to_entity()),
            UserContentDTO::Audio(audio) => DomainUserContent::Audio(audio.to_entity()),
            UserContentDTO::Video(video) => DomainUserContent::Video(video.to_entity()),
            UserContentDTO::Document(document) => {
                DomainUserContent::Document(document.to_entity())
            }
        }
    }

    pub fn from_entity(entity: DomainUserContent) -> Self {
        match entity {
            DomainUserContent::Text(text) => UserContentDTO::Text(text),
            DomainUserContent::Image(image) => {
                UserContentDTO::Image(ImageDTO::from_entity(image))
            }
            DomainUserContent::Audio(audio) => {
                UserContentDTO::Audio(AudioDTO::from_entity(audio))
            }
            DomainUserContent::Video(video) => {
                UserContentDTO::Video(VideoDTO::from_entity(video))
            }
            DomainUserContent::Document(document) => {
                UserContentDTO::Document(DocumentDTO::from_entity(document))
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct CreateUserMessageRequestDTO {
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,
    pub content: Vec<UserContentDTO>,
}

impl CreateUserMessageRequestDTO {
    pub fn to_params(&self) -> CreateUserMessageParams {
        CreateUserMessageParams {
            name: None,
            id: self.id,
            content: self
                .content
                .iter()
                .map(UserContentDTO::to_entity)
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct CreateToolMessageRequestDTO {
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,
    pub tool_call_id: String,
    pub parent_message_id: Option<Uuid>,
    pub content: Option<String>,
    pub error: Option<String>,
}

impl CreateToolMessageRequestDTO {
    pub fn to_params(&self) -> CreateToolMessageParams {
        CreateToolMessageParams {
            name: None,
            id: self.id,
            tool_call_id: self.tool_call_id.clone(),
            parent_message_id: self.parent_message_id,
            content: self.content.clone(),
            error: self.error.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "role", rename_all = "snake_case")]
pub enum MessageResponseDTO {
    System(SystemMessageResponseDTO),
    User(UserMessageResponseDTO),
    Assistant(AssistantMessageResponseDTO),
    Tool(ToolMessageResponseDTO),
    Reasoning(ReasoningMessageResponseDTO),
}

impl MessageResponseDTO {
    pub fn from_response(response: MessageResponse) -> Self {
        match response {
            MessageResponse::System(res) => {
                MessageResponseDTO::System(SystemMessageResponseDTO::from_response(res))
            }
            MessageResponse::User(res) => {
                MessageResponseDTO::User(UserMessageResponseDTO::from_response(res))
            }
            MessageResponse::Assistant(res) => {
                MessageResponseDTO::Assistant(AssistantMessageResponseDTO::from_response(res))
            }
            MessageResponse::Tool(res) => {
                MessageResponseDTO::Tool(ToolMessageResponseDTO::from_response(res))
            }
            MessageResponse::Reasoning(res) => {
                MessageResponseDTO::Reasoning(ReasoningMessageResponseDTO::from_response(res))
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SystemMessageResponseDTO {
    pub id: Uuid,
    pub session_id: Uuid,
    pub run_id: Option<Uuid>,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

impl SystemMessageResponseDTO {
    pub fn from_response(response: SystemMessageResponse) -> Self {
        Self {
            id: response.id,
            session_id: response.session_id,
            run_id: response.run_id,
            content: response.content,
            created_at: response.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserMessageResponseDTO {
    pub id: Uuid,
    pub session_id: Uuid,
    pub run_id: Option<Uuid>,
    pub content: Vec<UserContentDTO>,
    pub created_at: DateTime<Utc>,
}

impl UserMessageResponseDTO {
    pub fn from_response(response: UserMessageResponse) -> Self {
        Self {
            id: response.id,
            session_id: response.session_id,
            run_id: response.run_id,
            content: response
                .content
                .into_iter()
                .map(UserContentDTO::from_entity)
                .collect(),
            created_at: response.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AssistantMessageResponseDTOToolCall {
    pub id: String,
    pub name: String,
    pub arguments: Value,
}

impl AssistantMessageResponseDTOToolCall {
    pub fn from_tool_call(tool_call: ToolCall) -> Self {
        Self {
            id: tool_call.id,
            name: tool_call.name,
            arguments: tool_call.arguments,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AssistantMessageResponseDTO {
    pub id: Uuid,
    pub session_id: Uuid,
    pub run_id: Uuid,
    pub content: Option<String>,
    pub tool_calls: Option<Vec<AssistantMessageResponseDTOToolCall>>,
    pub created_at: DateTime<Utc>,
}

impl AssistantMessageResponseDTO {
    pub fn from_response(response: AssistantMessageResponse) -> Self {
        Self {
            id: response.id,
            session_id: response.session_id,
            run_id: response.run_id,
            content: response.content,
            tool_calls: response.tool_calls.map(|calls| {
                calls
                    .into_iter()
                    .map(AssistantMessageResponseDTOToolCall::from_tool_call)
                    .collect()
            }),
            created_at: response.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ToolMessageResponseDTO {
    pub id: Uuid,
    pub session_id: Uuid,
    pub run_id: Option<Uuid>,
    pub tool_call_id: String,
    pub content: Option<String>,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl ToolMessageResponseDTO {
    pub fn from_response(response: ToolMessageResponse) -> Self {
        Self {
            id: response.id,
            session_id: response.session_id,
            run_id: response.run_id,
            tool_call_id: response.tool_call_id,
            content: response.content,
            error: response.error,
            created_at: response.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReasoningMessageResponseDTO {
    pub id: Uuid,
    pub session_id: Uuid,
    pub run_id: Uuid,
    pub content: String,
    pub signature: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl ReasoningMessageResponseDTO {
    pub fn from_response(response: ReasoningMessageResponse) -> Self {
        Self {
            id: response.id,
            session_id: response.session_id,
            run_id: response.run_id,
            content: response.content,
            signature: response.signature,
            created_at: response.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum MessageRoleDTO {
    System,
    User,
    Assistant,
    Tool,
    Reasoning,
}

impl MessageRoleDTO {
    pub fn into_role(self) -> MessageRole {
        match self {
            MessageRoleDTO::System => MessageRole::System,
            MessageRoleDTO::User => MessageRole::User,
            MessageRoleDTO::Assistant => MessageRole::Assistant,
            MessageRoleDTO::Tool => MessageRole::Tool,
            MessageRoleDTO::Reasoning => MessageRole::Reasoning,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, IntoParams)]
pub struct FindMessageEndpointParams {
    #[param(minimum = 1, maximum = 100)]
    #[validate(range(min = 1, max = 100))]
    pub per_page: Option<u64>,
    pub page: Option<String>,
    pub ids: Option<Vec<Uuid>>,
    pub run_ids: Option<Vec<Uuid>>,
    pub roles: Option<Vec<MessageRoleDTO>>,
    pub created_before: Option<DateTime<Utc>>,
    pub created_after: Option<DateTime<Utc>>,
}

impl FindMessageEndpointParams {
    pub fn to_params(
        &self,
        tenant_id: impl Into<String>,
        session_id: impl Into<Uuid>,
    ) -> FindMessageParams {
        FindMessageParams {
            per_page: self.per_page,
            page: self.page.clone(),
            tenant_ids: Some(vec![tenant_id.into()]),
            session_ids: Some(vec![session_id.into()]),
            ids: self.ids.clone(),
            run_ids: self.run_ids.clone(),
            roles: self.roles.clone().map(|roles| {
                roles
                    .into_iter()
                    .map(MessageRoleDTO::into_role)
                    .collect()
            }),
            created_before: self.created_before,
            created_after: self.created_after,
        }
    }
}
