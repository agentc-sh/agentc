// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use json_patch::{AddOperation, PatchOperation};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json, to_value};

use agentc_agent::tools::{
    activity::ActivityDelta, errors::ToolError, types::ToolExecutionContext,
};

use crate::{
    protocol::{
        Artifact, CancelTaskRequest, GetTaskRequest, Message, Part, PartContent, Role,
        SendMessageConfiguration, SendMessageRequest, StreamResponse, Task,
        TaskArtifactUpdateEvent, TaskId, TaskState, TaskStatusUpdateEvent,
    },
    tools::target::A2aToolTarget,
};

pub struct A2aStreamActivity;

impl A2aStreamActivity {
    pub fn delta(
        target: &A2aToolTarget,
        response: &StreamResponse,
    ) -> Result<ActivityDelta, ToolError> {
        Ok(ActivityDelta {
            activity_type: Self::activity_type(response).to_string(),
            patch: Self::patch(target, response)?,
        })
    }

    pub fn is_terminal(response: &StreamResponse) -> bool {
        match response {
            StreamResponse::Task(task) => task.status.state.is_terminal(),
            StreamResponse::StatusUpdate(update) => update.status.state.is_terminal(),
            StreamResponse::Message(_) | StreamResponse::ArtifactUpdate(_) => false,
        }
    }

    fn activity_type(response: &StreamResponse) -> &'static str {
        match response {
            StreamResponse::Task(_) => "a2a_task",
            StreamResponse::Message(_) => "a2a_message",
            StreamResponse::StatusUpdate(_) => "a2a_task_status",
            StreamResponse::ArtifactUpdate(_) => "a2a_artifact",
        }
    }

    fn patch(
        target: &A2aToolTarget,
        response: &StreamResponse,
    ) -> Result<Vec<PatchOperation>, ToolError> {
        let mut patch = vec![
            Self::add("/target_id", json!(&target.id))?,
            Self::add("/event", to_value(response)?)?,
        ];

        match response {
            StreamResponse::Task(task) => Self::append_task(&mut patch, task)?,
            StreamResponse::Message(message) => Self::append_message(&mut patch, message)?,
            StreamResponse::StatusUpdate(update) => Self::append_status(&mut patch, update)?,
            StreamResponse::ArtifactUpdate(update) => Self::append_artifact(&mut patch, update)?,
        }

        Ok(patch)
    }

    fn append_task(patch: &mut Vec<PatchOperation>, task: &Task) -> Result<(), ToolError> {
        patch.push(Self::add("/task_id", json!(task.id.to_string()))?);
        patch.push(Self::add("/context_id", json!(&task.context_id))?);
        patch.push(Self::add("/state", Self::state(&task.status.state)?)?);

        if let Some(message) = &task.status.message {
            patch.push(Self::add("/latest_message", to_value(message)?)?);
        }

        if let Some(artifacts) = &task.artifacts {
            patch.push(Self::add("/artifacts", to_value(artifacts)?)?);
        }

        Ok(())
    }

    fn append_message(patch: &mut Vec<PatchOperation>, message: &Message) -> Result<(), ToolError> {
        if let Some(task_id) = &message.task_id {
            patch.push(Self::add("/task_id", json!(task_id.to_string()))?);
        }

        if let Some(context_id) = &message.context_id {
            patch.push(Self::add("/context_id", json!(context_id))?);
        }

        patch.push(Self::add("/latest_message", to_value(message)?)?);

        Ok(())
    }

    fn append_status(
        patch: &mut Vec<PatchOperation>,
        update: &TaskStatusUpdateEvent,
    ) -> Result<(), ToolError> {
        patch.push(Self::add("/task_id", json!(update.task_id.to_string()))?);
        patch.push(Self::add("/context_id", json!(&update.context_id))?);
        patch.push(Self::add("/state", Self::state(&update.status.state)?)?);

        if let Some(message) = &update.status.message {
            patch.push(Self::add("/latest_message", to_value(message)?)?);
        }

        Ok(())
    }

    fn append_artifact(
        patch: &mut Vec<PatchOperation>,
        update: &TaskArtifactUpdateEvent,
    ) -> Result<(), ToolError> {
        patch.push(Self::add("/task_id", json!(update.task_id.to_string()))?);
        patch.push(Self::add("/context_id", json!(&update.context_id))?);
        patch.push(Self::add("/artifact", to_value(&update.artifact)?)?);

        Ok(())
    }

    fn state(state: &TaskState) -> Result<Value, ToolError> {
        Ok(to_value(state)?)
    }

    fn add(path: &str, value: Value) -> Result<PatchOperation, ToolError> {
        Ok(PatchOperation::Add(AddOperation {
            path: path
                .try_into()
                .map_err(|_| ToolError::execution_error("a2a_stream", "invalid activity path"))?,
            value,
        }))
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct A2aStreamTaskToolResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<TaskId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<TaskState>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub artifacts: Vec<Artifact>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<Message>,
}

impl A2aStreamTaskToolResult {
    pub fn from_events(events: Vec<StreamResponse>) -> Self {
        let mut result = Self::default();

        for event in events {
            match event {
                StreamResponse::Task(task) => {
                    result.task_id = Some(task.id);
                    result.context_id = Some(task.context_id);
                    result.state = Some(task.status.state);

                    if task.status.message.is_some() {
                        result.message = task.status.message;
                    }

                    for artifact in task.artifacts.into_iter().flatten() {
                        result.merge_artifact(artifact, false);
                    }
                }
                StreamResponse::StatusUpdate(update) => {
                    result.task_id = Some(update.task_id);
                    result.context_id = Some(update.context_id);
                    result.state = Some(update.status.state);

                    if update.status.message.is_some() {
                        result.message = update.status.message;
                    }
                }
                StreamResponse::ArtifactUpdate(update) => {
                    result.task_id = Some(update.task_id);
                    result.context_id = Some(update.context_id);
                    result.merge_artifact(update.artifact, update.append.unwrap_or(false));
                }
                StreamResponse::Message(message) => {
                    result.task_id = message
                        .task_id
                        .clone()
                        .or(result.task_id);
                    result.context_id = message
                        .context_id
                        .clone()
                        .or(result.context_id);
                    result.message = Some(message);
                }
            }
        }

        result
    }

    fn merge_artifact(&mut self, incoming: Artifact, append: bool) {
        match self
            .artifacts
            .iter_mut()
            .find(|artifact| artifact.artifact_id == incoming.artifact_id)
        {
            Some(existing) if append => {
                for part in incoming.parts {
                    Self::append_part(&mut existing.parts, part);
                }
            }
            Some(existing) => *existing = incoming,
            None => self.artifacts.push(incoming),
        }
    }

    fn append_part(parts: &mut Vec<Part>, part: Part) {
        if let PartContent::Text(next) = &part.content {
            if let Some(Part { content: PartContent::Text(existing), .. }) = parts.last_mut() {
                existing.push_str(next);
                return;
            }
        }

        parts.push(part);
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct A2aSendTaskToolInputMessage {
    pub text: Option<String>,
    pub data: Option<Value>,
}

impl A2aSendTaskToolInputMessage {
    pub(crate) fn into_parts(self) -> Result<Vec<Part>, ToolError> {
        if self.text.is_none() && self.data.is_none() {
            return Err(ToolError::invalid_args(
                "A2A message requires at least one of text or data",
            ));
        }

        Ok(self
            .text
            .map(Part::text)
            .into_iter()
            .chain(self.data.map(Part::data))
            .collect())
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct A2aSendTaskToolInput {
    pub message: A2aSendTaskToolInputMessage,
    pub context_id: Option<String>,
    pub task_id: Option<String>,
    pub metadata: Option<Value>,
    pub accepted_output_modes: Option<Vec<String>>,
    pub history_length: Option<i32>,
}

impl A2aSendTaskToolInput {
    pub(crate) fn into_request(
        self,
        target: &A2aToolTarget,
        context: &ToolExecutionContext,
        return_immediately: Option<bool>,
    ) -> Result<SendMessageRequest, ToolError> {
        let Self {
            message,
            context_id,
            task_id,
            metadata,
            accepted_output_modes,
            history_length,
        } = self;
        let accepted_output_modes = accepted_output_modes.or_else(|| {
            target
                .default_accepted_output_modes
                .clone()
        });

        Ok(SendMessageRequest {
            message: Message {
                context_id,
                task_id: task_id.map(TaskId::new),
                ..Message::new(Role::User, message.into_parts()?)
            },
            configuration: (accepted_output_modes.is_some()
                || history_length.is_some()
                || return_immediately.is_some())
            .then_some(SendMessageConfiguration {
                accepted_output_modes,
                task_push_notification_config: None,
                history_length,
                return_immediately,
            }),
            metadata: match metadata {
                Some(Value::Object(map)) => Some(map.into_iter().collect()),
                Some(_) => {
                    return Err(ToolError::invalid_args("A2A metadata must be a JSON object"));
                }
                None => None,
            },
            tenant: target.tenant_policy.resolve(context),
        })
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct A2aGetTaskToolInput {
    pub task_id: String,
    pub history_length: Option<i32>,
}

impl A2aGetTaskToolInput {
    pub(crate) fn into_request(
        self,
        target: &A2aToolTarget,
        context: &ToolExecutionContext,
    ) -> GetTaskRequest {
        GetTaskRequest {
            id: TaskId::new(self.task_id),
            history_length: self.history_length,
            tenant: target.tenant_policy.resolve(context),
        }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct A2aCancelTaskToolInput {
    pub task_id: String,
    pub metadata: Option<Value>,
}

impl A2aCancelTaskToolInput {
    pub(crate) fn into_request(
        self,
        target: &A2aToolTarget,
        context: &ToolExecutionContext,
    ) -> Result<CancelTaskRequest, ToolError> {
        Ok(CancelTaskRequest {
            id: TaskId::new(self.task_id),
            metadata: match self.metadata {
                Some(Value::Object(map)) => Some(map.into_iter().collect()),
                Some(_) => {
                    return Err(ToolError::invalid_args("A2A metadata must be a JSON object"));
                }
                None => None,
            },
            tenant: target.tenant_policy.resolve(context),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use agentc_agent::tools::{activity::ActivityDelta, types::ToolExecutionContext};
    use json_patch::PatchOperation;
    use serde_json::json;
    use uuid::Uuid;

    use crate::{
        client::{A2aClient, A2aClientConfig},
        protocol::{
            Artifact, ArtifactId, Message, Part, PartContent, Role, StreamResponse, Task,
            TaskArtifactUpdateEvent, TaskId, TaskState, TaskStatus, TaskStatusUpdateEvent,
        },
        tools::target::A2aTenantPolicy,
    };

    struct ToolTypesFixture;

    impl ToolTypesFixture {
        fn context() -> ToolExecutionContext {
            ToolExecutionContext {
                tenant_id: "parent-tenant".to_string(),
                session_id: Uuid::nil(),
                run_id: Uuid::nil(),
            }
        }

        fn target() -> A2aToolTarget {
            A2aToolTarget::builder()
                .id("planner")
                .client(
                    A2aClient::new(A2aClientConfig::new("http://localhost:8080"))
                        .expect("client config should be valid"),
                )
                .default_accepted_output_modes(["text/plain"])
                .build()
                .expect("target should build")
        }

        fn fixed_tenant_target() -> A2aToolTarget {
            A2aToolTarget::builder()
                .id("planner")
                .client(
                    A2aClient::new(A2aClientConfig::new("http://localhost:8080"))
                        .expect("client config should be valid"),
                )
                .tenant_policy(A2aTenantPolicy::Fixed("downstream".to_string()))
                .build()
                .expect("target should build")
        }

        fn send_input() -> A2aSendTaskToolInput {
            A2aSendTaskToolInput {
                message: A2aSendTaskToolInputMessage {
                    text: Some("plan this".to_string()),
                    data: None,
                },
                context_id: Some("context-1".to_string()),
                task_id: None,
                metadata: Some(json!({
                    "priority": "high",
                })),
                accepted_output_modes: None,
                history_length: Some(4),
            }
        }

        fn add_value<'a>(delta: &'a ActivityDelta, path: &str) -> Option<&'a Value> {
            delta
                .patch
                .iter()
                .find_map(|operation| match operation {
                    PatchOperation::Add(operation) if operation.path.to_string() == path => {
                        Some(&operation.value)
                    }
                    _ => None,
                })
        }

        fn submitted(task_id: &TaskId) -> StreamResponse {
            StreamResponse::Task(Task {
                id: task_id.clone(),
                context_id: "context-1".to_string(),
                status: TaskStatus {
                    state: TaskState::Submitted,
                    message: None,
                    timestamp: None,
                },
                artifacts: None,
                history: None,
                metadata: None,
            })
        }

        fn status(task_id: &TaskId, state: TaskState) -> StreamResponse {
            StreamResponse::StatusUpdate(TaskStatusUpdateEvent {
                task_id: task_id.clone(),
                context_id: "context-1".to_string(),
                status: TaskStatus { state, message: None, timestamp: None },
                metadata: None,
            })
        }

        fn artifact_delta(
            task_id: &TaskId,
            artifact_id: &str,
            part: Part,
            append: bool,
        ) -> StreamResponse {
            StreamResponse::ArtifactUpdate(TaskArtifactUpdateEvent {
                task_id: task_id.clone(),
                context_id: "context-1".to_string(),
                artifact: Artifact {
                    artifact_id: ArtifactId::new(artifact_id),
                    name: None,
                    description: None,
                    parts: vec![part],
                    metadata: None,
                    extensions: None,
                },
                append: Some(append),
                last_chunk: None,
                metadata: None,
            })
        }
    }

    #[test]
    fn send_input_builds_user_request_with_target_defaults_and_inherited_tenant() {
        let request = ToolTypesFixture::send_input()
            .into_request(&ToolTypesFixture::target(), &ToolTypesFixture::context(), Some(true))
            .expect("request should build");

        assert_eq!(request.message.context_id.as_deref(), Some("context-1"));
        assert_eq!(request.tenant.as_deref(), Some("parent-tenant"));
        assert_eq!(
            request
                .configuration
                .as_ref()
                .and_then(|config| config.accepted_output_modes.as_ref())
                .map(Vec::len),
            Some(1)
        );
        assert_eq!(
            request
                .configuration
                .as_ref()
                .and_then(|config| config.accepted_output_modes.as_ref())
                .and_then(|modes| modes.first())
                .map(String::as_str),
            Some("text/plain")
        );
        assert_eq!(
            request
                .configuration
                .as_ref()
                .and_then(|config| config.history_length),
            Some(4)
        );
        assert_eq!(
            request
                .configuration
                .as_ref()
                .and_then(|config| config.return_immediately),
            Some(true)
        );

        match request.message.parts.as_slice() {
            [Part { content: PartContent::Text(text), .. }] => {
                assert_eq!(text, "plan this");
            }
            _ => panic!("expected one text part"),
        }
    }

    #[test]
    fn get_task_uses_target_tenant_policy() {
        assert_eq!(
            A2aGetTaskToolInput {
                task_id: "task-1".to_string(),
                history_length: Some(2),
            }
            .into_request(&ToolTypesFixture::fixed_tenant_target(), &ToolTypesFixture::context(),)
            .tenant
            .as_deref(),
            Some("downstream")
        );
    }

    #[test]
    fn cancel_task_rejects_non_object_metadata() {
        assert!(
            A2aCancelTaskToolInput {
                task_id: "task-1".to_string(),
                metadata: Some(json!("invalid")),
            }
            .into_request(&ToolTypesFixture::target(), &ToolTypesFixture::context(),)
            .is_err()
        );
    }

    #[test]
    fn stream_activity_status_delta_exposes_task_identity_and_state() {
        let delta = A2aStreamActivity::delta(
            &ToolTypesFixture::target(),
            &StreamResponse::StatusUpdate(TaskStatusUpdateEvent {
                task_id: TaskId::new("task-1"),
                context_id: "context-1".to_string(),
                status: TaskStatus {
                    state: TaskState::Working,
                    message: None,
                    timestamp: None,
                },
                metadata: None,
            }),
        )
        .expect("delta should build");

        assert_eq!(delta.activity_type, "a2a_task_status");
        assert_eq!(ToolTypesFixture::add_value(&delta, "/target_id"), Some(&json!("planner")));
        assert_eq!(ToolTypesFixture::add_value(&delta, "/task_id"), Some(&json!("task-1")));
        assert_eq!(ToolTypesFixture::add_value(&delta, "/context_id"), Some(&json!("context-1")));
        assert_eq!(
            ToolTypesFixture::add_value(&delta, "/state"),
            Some(&json!("TASK_STATE_WORKING"))
        );
    }

    #[test]
    fn stream_activity_identifies_terminal_task_events() {
        assert!(A2aStreamActivity::is_terminal(&StreamResponse::Task(Task {
            id: TaskId::new("task-1"),
            context_id: "context-1".to_string(),
            status: TaskStatus {
                state: TaskState::Completed,
                message: None,
                timestamp: None,
            },
            artifacts: None,
            history: None,
            metadata: None,
        })));
    }

    #[test]
    fn from_events_merges_appended_artifacts_and_uses_terminal_state() {
        let task_id = TaskId::new("task-1");

        let result = A2aStreamTaskToolResult::from_events(vec![
            ToolTypesFixture::submitted(&task_id),
            ToolTypesFixture::status(&task_id, TaskState::Working),
            ToolTypesFixture::artifact_delta(&task_id, "response", Part::text("Hello, "), true),
            ToolTypesFixture::artifact_delta(&task_id, "response", Part::text("world!"), true),
            ToolTypesFixture::artifact_delta(
                &task_id,
                "state",
                Part::data(json!({ "answer": 42 })),
                false,
            ),
            ToolTypesFixture::status(&task_id, TaskState::Completed),
        ]);

        assert_eq!(result.task_id.as_ref(), Some(&task_id));
        assert_eq!(result.context_id.as_deref(), Some("context-1"));
        assert_eq!(result.state, Some(TaskState::Completed));
        assert_eq!(result.artifacts.len(), 2);

        let response = result
            .artifacts
            .iter()
            .find(|artifact| artifact.artifact_id == ArtifactId::new("response"))
            .expect("response artifact should exist");

        assert_eq!(response.parts.len(), 1);
        assert_eq!(response.parts[0].as_text(), Some("Hello, world!"));

        let state = result
            .artifacts
            .iter()
            .find(|artifact| artifact.artifact_id == ArtifactId::new("state"))
            .expect("state artifact should exist");

        assert!(matches!(state.parts[0].content, PartContent::Data(_)));
    }

    #[test]
    fn from_events_replaces_artifact_when_update_is_not_append() {
        let task_id = TaskId::new("task-2");

        let result = A2aStreamTaskToolResult::from_events(vec![
            ToolTypesFixture::artifact_delta(&task_id, "doc", Part::text("draft"), true),
            ToolTypesFixture::artifact_delta(&task_id, "doc", Part::text("final"), false),
        ]);

        assert_eq!(result.artifacts.len(), 1);
        assert_eq!(result.artifacts[0].parts.len(), 1);
        assert_eq!(result.artifacts[0].parts[0].as_text(), Some("final"));
    }

    #[test]
    fn from_events_captures_message_identity_and_content() {
        let result = A2aStreamTaskToolResult::from_events(vec![StreamResponse::Message(Message {
            context_id: Some("context-9".to_string()),
            task_id: Some(TaskId::new("task-9")),
            ..Message::new(Role::Agent, vec![Part::text("done")])
        })]);

        assert_eq!(result.task_id, Some(TaskId::new("task-9")));
        assert_eq!(result.context_id.as_deref(), Some("context-9"));
        assert_eq!(result.state, None);
        assert_eq!(
            result
                .message
                .as_ref()
                .and_then(Message::text),
            Some("done")
        );
    }
}
