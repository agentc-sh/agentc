// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as DeError};
use serde_json::Value;
use std::collections::HashMap;

use crate::protocol::{artifact::Artifact, ids::TaskId, message::Message};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
pub enum TaskState {
    #[default]
    Unspecified,
    Submitted,
    Working,
    Completed,
    Failed,
    Canceled,
    InputRequired,
    Rejected,
    AuthRequired,
}

impl TaskState {
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Canceled | Self::Rejected)
    }
}

impl Serialize for TaskState {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(match self {
            Self::Unspecified => "TASK_STATE_UNSPECIFIED",
            Self::Submitted => "TASK_STATE_SUBMITTED",
            Self::Working => "TASK_STATE_WORKING",
            Self::Completed => "TASK_STATE_COMPLETED",
            Self::Failed => "TASK_STATE_FAILED",
            Self::Canceled => "TASK_STATE_CANCELED",
            Self::InputRequired => "TASK_STATE_INPUT_REQUIRED",
            Self::Rejected => "TASK_STATE_REJECTED",
            Self::AuthRequired => "TASK_STATE_AUTH_REQUIRED",
        })
    }
}

impl<'de> Deserialize<'de> for TaskState {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match String::deserialize(deserializer)?.as_str() {
            "TASK_STATE_SUBMITTED" => Ok(Self::Submitted),
            "TASK_STATE_WORKING" => Ok(Self::Working),
            "TASK_STATE_COMPLETED" => Ok(Self::Completed),
            "TASK_STATE_FAILED" => Ok(Self::Failed),
            "TASK_STATE_CANCELED" => Ok(Self::Canceled),
            "TASK_STATE_INPUT_REQUIRED" => Ok(Self::InputRequired),
            "TASK_STATE_REJECTED" => Ok(Self::Rejected),
            "TASK_STATE_AUTH_REQUIRED" => Ok(Self::AuthRequired),
            "TASK_STATE_UNSPECIFIED" | "" => Ok(Self::Unspecified),
            other => Err(DeError::unknown_variant(
                other,
                &[
                    "TASK_STATE_SUBMITTED",
                    "TASK_STATE_WORKING",
                    "TASK_STATE_COMPLETED",
                    "TASK_STATE_FAILED",
                    "TASK_STATE_CANCELED",
                    "TASK_STATE_INPUT_REQUIRED",
                    "TASK_STATE_REJECTED",
                    "TASK_STATE_AUTH_REQUIRED",
                    "TASK_STATE_UNSPECIFIED",
                ],
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct TaskStatus {
    pub state: TaskState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<Message>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: TaskId,
    pub context_id: String,
    pub status: TaskStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifacts: Option<Vec<Artifact>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub history: Option<Vec<Message>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, Value>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct GetTaskRequest {
    pub id: TaskId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub history_length: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenant: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct CancelTaskRequest {
    pub id: TaskId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenant: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct AuthenticationInfo {
    pub scheme: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credentials: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct TaskPushNotificationConfig {
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "TaskId::is_empty")]
    pub task_id: TaskId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authentication: Option<AuthenticationInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenant: Option<String>,
}
