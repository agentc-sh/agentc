// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use serde::{
    Deserialize, Deserializer, Serialize, Serializer, de::Error as DeError, ser::SerializeMap,
};
use serde_json::{Value, from_value};
use std::collections::HashMap;

use crate::protocol::{
    artifact::Artifact,
    ids::TaskId,
    message::Message,
    task::{Task, TaskStatus},
};

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
pub enum StreamResponse {
    Task(Task),
    Message(Message),
    StatusUpdate(TaskStatusUpdateEvent),
    ArtifactUpdate(TaskArtifactUpdateEvent),
}

impl Serialize for StreamResponse {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(1))?;

        match self {
            Self::Task(task) => map.serialize_entry("task", task)?,
            Self::Message(message) => map.serialize_entry("message", message)?,
            Self::StatusUpdate(status) => map.serialize_entry("statusUpdate", status)?,
            Self::ArtifactUpdate(artifact) => map.serialize_entry("artifactUpdate", artifact)?,
        }

        map.end()
    }
}

impl<'de> Deserialize<'de> for StreamResponse {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = HashMap::<String, Value>::deserialize(deserializer)?;

        if let Some(task) = raw.get("task") {
            return Ok(Self::Task(from_value(task.clone()).map_err(DeError::custom)?));
        }

        if let Some(message) = raw.get("message") {
            return Ok(Self::Message(from_value(message.clone()).map_err(DeError::custom)?));
        }

        if let Some(status) = raw.get("statusUpdate") {
            return Ok(Self::StatusUpdate(from_value(status.clone()).map_err(DeError::custom)?));
        }

        if let Some(artifact) = raw.get("artifactUpdate") {
            return Ok(Self::ArtifactUpdate(
                from_value(artifact.clone()).map_err(DeError::custom)?,
            ));
        }

        Err(DeError::custom("unknown StreamResponse variant"))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct TaskStatusUpdateEvent {
    pub task_id: TaskId,
    pub context_id: String,
    pub status: TaskStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, Value>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct TaskArtifactUpdateEvent {
    pub task_id: TaskId,
    pub context_id: String,
    pub artifact: Artifact,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub append: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_chunk: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, Value>>,
}
