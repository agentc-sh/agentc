// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use agentc_agent::types::conversion::{FromModelType, ToModelType};
use agentc_model::types::message::{
    UserContent as ModelUserContent, UserMessage as ModelUserMessage,
};

use super::{Audio, Document, Image, MediaSource, MessageRole, Video};

/// A content block in a user message.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum UserContent {
    Text(String),
    Image(Image),
    Audio(Audio),
    Video(Video),
    Document(Document),
}

impl UserContent {
    pub fn text(content: impl Into<String>) -> Self {
        Self::Text(content.into())
    }

    pub fn image(source: impl Into<MediaSource>, media_type: impl Into<String>) -> Self {
        Self::Image(Image {
            source: source.into(),
            media_type: media_type.into(),
        })
    }

    pub fn audio(source: impl Into<MediaSource>, media_type: impl Into<String>) -> Self {
        Self::Audio(Audio {
            source: source.into(),
            media_type: media_type.into(),
        })
    }

    pub fn video(source: impl Into<MediaSource>, media_type: impl Into<String>) -> Self {
        Self::Video(Video {
            source: source.into(),
            media_type: media_type.into(),
        })
    }

    pub fn document(source: impl Into<MediaSource>, media_type: impl Into<String>) -> Self {
        Self::Document(Document {
            source: source.into(),
            media_type: media_type.into(),
        })
    }

    pub fn as_text(&self) -> Option<&str> {
        match self {
            UserContent::Text(text) => Some(text),
            _ => None,
        }
    }
}

impl From<String> for UserContent {
    fn from(content: String) -> Self {
        Self::text(content)
    }
}

impl From<&str> for UserContent {
    fn from(content: &str) -> Self {
        Self::text(content)
    }
}

impl From<Image> for UserContent {
    fn from(image: Image) -> Self {
        Self::Image(image)
    }
}

impl From<Audio> for UserContent {
    fn from(audio: Audio) -> Self {
        Self::Audio(audio)
    }
}

impl From<Video> for UserContent {
    fn from(video: Video) -> Self {
        Self::Video(video)
    }
}

impl From<Document> for UserContent {
    fn from(document: Document) -> Self {
        Self::Document(document)
    }
}

impl ToModelType for UserContent {
    type ModelType = ModelUserContent;

    fn to_model_type(&self) -> Self::ModelType {
        match self {
            UserContent::Text(text) => ModelUserContent::Text(text.clone()),
            UserContent::Image(image) => ModelUserContent::Image(image.to_model_type()),
            UserContent::Audio(audio) => ModelUserContent::Audio(audio.to_model_type()),
            UserContent::Video(video) => ModelUserContent::Video(video.to_model_type()),
            UserContent::Document(document) => ModelUserContent::Document(document.to_model_type()),
        }
    }
}

impl FromModelType for UserContent {
    type ModelType = ModelUserContent;
    type Output = Option<Self>;

    fn from_model_type(model: Self::ModelType) -> Self::Output {
        match model {
            ModelUserContent::Text(text) => Some(UserContent::Text(text)),
            ModelUserContent::Image(image) => {
                Some(UserContent::Image(Image::from_model_type(image)))
            }
            ModelUserContent::Audio(audio) => {
                Some(UserContent::Audio(Audio::from_model_type(audio)))
            }
            ModelUserContent::Video(video) => {
                Some(UserContent::Video(Video::from_model_type(video)))
            }
            ModelUserContent::Document(document) => {
                Some(UserContent::Document(Document::from_model_type(document)))
            }
            ModelUserContent::ToolResult(_) => None,
        }
    }
}

/// A user message.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UserMessage {
    pub id: Uuid,
    pub tenant_id: String,
    pub session_id: Uuid,
    pub run_id: Option<Uuid>,
    pub checkpoint_id: Option<Uuid>,
    pub content: Vec<UserContent>,
    pub name: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl UserMessage {
    /// Creates a user message containing one text block.
    pub fn new(content: impl Into<String>) -> Self {
        Self::from_content([UserContent::text(content)])
    }

    /// Creates a user message from ordered content blocks.
    pub fn from_content(content: impl IntoIterator<Item = UserContent>) -> Self {
        Self {
            id: Uuid::new_v4(),
            tenant_id: String::new(),
            session_id: Uuid::nil(),
            run_id: None,
            checkpoint_id: None,
            content: content.into_iter().collect(),
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

    pub fn with_checkpoint_id(mut self, checkpoint_id: impl Into<Uuid>) -> Self {
        self.checkpoint_id = Some(checkpoint_id.into());
        self
    }

    /// Replaces the message's ordered content blocks.
    pub fn with_content(mut self, content: impl IntoIterator<Item = UserContent>) -> Self {
        self.content = content.into_iter().collect();
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
            content: self
                .content
                .iter()
                .map(UserContent::to_model_type)
                .collect(),
        }
    }
}

impl FromModelType for UserMessage {
    type ModelType = ModelUserMessage;
    type Output = Self;

    fn from_model_type(model: Self::ModelType) -> Self::Output {
        Self::from_content(
            model
                .content
                .into_iter()
                .filter_map(UserContent::from_model_type),
        )
    }
}
