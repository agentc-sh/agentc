// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use base64::prelude::*;
use serde::{
    Deserialize, Deserializer, Serialize, Serializer, de::Error as DeError, ser::SerializeMap,
};
use serde_json::{Value, from_value};
use std::collections::HashMap;
use uuid::Uuid;

use crate::protocol::{
    ids::TaskId,
    task::{Task, TaskPushNotificationConfig},
};

/// Identifies the sender of a message.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
pub enum Role {
    #[default]
    Unspecified,
    User,
    Agent,
}

impl Serialize for Role {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(match self {
            Role::Unspecified => "ROLE_UNSPECIFIED",
            Role::User => "ROLE_USER",
            Role::Agent => "ROLE_AGENT",
        })
    }
}

impl<'de> Deserialize<'de> for Role {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match String::deserialize(deserializer)?.as_str() {
            "ROLE_USER" => Ok(Role::User),
            "ROLE_AGENT" => Ok(Role::Agent),
            "ROLE_UNSPECIFIED" | "" => Ok(Role::Unspecified),
            other => Err(DeError::unknown_variant(
                other,
                &["ROLE_USER", "ROLE_AGENT", "ROLE_UNSPECIFIED"],
            )),
        }
    }
}

/// The content of a message or artifact part.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
pub enum PartContent {
    Text(String),
    Raw(Vec<u8>),
    Url(String),
    Data(Value),
}

/// A content part of a message or artifact.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
pub struct Part {
    pub content: PartContent,
    pub filename: Option<String>,
    pub media_type: Option<String>,
    pub metadata: Option<HashMap<String, Value>>,
}

impl Part {
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            content: PartContent::Text(text.into()),
            filename: None,
            media_type: None,
            metadata: None,
        }
    }

    pub fn raw(data: Vec<u8>) -> Self {
        Self {
            content: PartContent::Raw(data),
            filename: None,
            media_type: None,
            metadata: None,
        }
    }

    pub fn url(url: impl Into<String>) -> Self {
        Self {
            content: PartContent::Url(url.into()),
            filename: None,
            media_type: None,
            metadata: None,
        }
    }

    pub fn data(value: Value) -> Self {
        Self {
            content: PartContent::Data(value),
            filename: None,
            media_type: None,
            metadata: None,
        }
    }

    pub fn with_media_type(mut self, media_type: impl Into<String>) -> Self {
        self.media_type = Some(media_type.into());
        self
    }

    pub fn with_filename(mut self, filename: impl Into<String>) -> Self {
        self.filename = Some(filename.into());
        self
    }

    pub fn as_text(&self) -> Option<&str> {
        if let PartContent::Text(text) = &self.content {
            Some(text)
        } else {
            None
        }
    }
}

impl Serialize for Part {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(None)?;

        match &self.content {
            PartContent::Text(text) => map.serialize_entry("text", text)?,
            PartContent::Raw(raw) => map.serialize_entry("raw", &BASE64_STANDARD.encode(raw))?,
            PartContent::Url(url) => map.serialize_entry("url", url)?,
            PartContent::Data(data) => map.serialize_entry("data", data)?,
        }

        if let Some(filename) = &self.filename {
            map.serialize_entry("filename", filename)?;
        }

        if let Some(media_type) = &self.media_type {
            map.serialize_entry("mediaType", media_type)?;
        }

        if let Some(metadata) = &self.metadata {
            if !metadata.is_empty() {
                map.serialize_entry("metadata", metadata)?;
            }
        }

        map.end()
    }
}

impl<'de> Deserialize<'de> for Part {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = HashMap::<String, Value>::deserialize(deserializer)?;

        let content = if let Some(Value::String(text)) = raw.get("text") {
            PartContent::Text(text.clone())
        } else if let Some(Value::String(raw)) = raw.get("raw") {
            PartContent::Raw(
                BASE64_STANDARD
                    .decode(raw)
                    .map_err(DeError::custom)?,
            )
        } else if let Some(Value::String(url)) = raw.get("url") {
            PartContent::Url(url.clone())
        } else if let Some(data) = raw.get("data") {
            PartContent::Data(data.clone())
        } else {
            return Err(DeError::custom("Part must have one of: text, raw, url, data"));
        };

        Ok(Self {
            content,
            filename: raw
                .get("filename")
                .and_then(Value::as_str)
                .map(String::from),
            media_type: raw
                .get("mediaType")
                .and_then(Value::as_str)
                .map(String::from),
            metadata: raw
                .get("metadata")
                .and_then(|value| from_value(value.clone()).ok()),
        })
    }
}

/// A single message in a conversation between user and agent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub message_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_id: Option<TaskId>,
    pub role: Role,
    pub parts: Vec<Part>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extensions: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_task_ids: Option<Vec<TaskId>>,
}

impl Message {
    pub fn new(role: Role, parts: Vec<Part>) -> Self {
        Self {
            message_id: Uuid::new_v4().to_string(),
            context_id: None,
            task_id: None,
            role,
            parts,
            metadata: None,
            extensions: None,
            reference_task_ids: None,
        }
    }

    pub fn text(&self) -> Option<&str> {
        self.parts
            .iter()
            .find_map(Part::as_text)
    }
}

/// Configuration for `SendMessage` and `SendStreamingMessage`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct SendMessageConfiguration {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accepted_output_modes: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(alias = "pushNotificationConfig")]
    pub task_push_notification_config: Option<TaskPushNotificationConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub history_length: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub return_immediately: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct SendMessageRequest {
    pub message: Message,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub configuration: Option<SendMessageConfiguration>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenant: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
pub enum SendMessageResponse {
    Task(Task),
    Message(Message),
}

impl Serialize for SendMessageResponse {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(1))?;

        match self {
            Self::Task(task) => map.serialize_entry("task", task)?,
            Self::Message(message) => map.serialize_entry("message", message)?,
        }

        map.end()
    }
}

impl<'de> Deserialize<'de> for SendMessageResponse {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = HashMap::<String, Value>::deserialize(deserializer)?;

        if let Some(task) = raw.get("task") {
            return Ok(Self::Task(from_value(task.clone()).map_err(DeError::custom)?));
        }

        if let Some(message) = raw.get("message") {
            return Ok(Self::Message(from_value(message.clone()).map_err(DeError::custom)?));
        }

        Err(DeError::custom("SendMessageResponse must have 'task' or 'message'"))
    }
}
