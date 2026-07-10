// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use chrono::Utc;
use futures::StreamExt;
use serde_json::{
    json,
    Value,
};
use uuid::Uuid;

use agentc_agent::types::identity::AgentIdentity;
use agentc_domain::{
    repository::scope::RepoScopeFactory,
    types::RunStatus,
};
use agentc_protocol_a2a::{
    protocol::{
        AgentCapabilities,
        AgentCard,
        AgentInterface,
        AgentSkill,
        Artifact,
        ArtifactId,
        CancelTaskRequest,
        GetTaskRequest,
        Message,
        Part,
        PartContent,
        Role,
        SendMessageRequest,
        SendMessageResponse,
        StreamResponse,
        Task,
        TaskArtifactUpdateEvent,
        TaskId,
        TaskState,
        TaskStatus,
        TaskStatusUpdateEvent,
    },
    traits::{
        A2aRunCancel,
        A2aService,
        A2aStream,
        FromA2aType,
        ToA2aType,
    },
};
use agentc_domain_sql::scope::SqlScopeFactoryError;
use agentc_http::errors::ApiError;

use crate::{
    service::{
        ApplicationService,
        operations::run::RunOperations,
        types::{
            message::{
                CreateMessageParams,
                CreateUserMessageParams,
            },
            run::{
                RunEvent,
                RunParams,
                RunResponse,
            },
        },
    },
    repository::state_snapshot::{
        params::FindStateSnapshotParams,
        traits::{StateSnapshotRepository, StateSnapshotRepoProvider},
    },
    types::{
        message::UserContent,
        state_snapshot::StateSnapshot,
    },
};

struct A2aRunInput {
    context_id: String,
    run_id: Uuid,
    params: RunParams,
}

struct ApplicationServiceA2aCancel {
    service: ApplicationService,
    tenant_id: String,
    run_id: Uuid,
}

struct A2aAgentCardInput<'a> {
    identity: &'a AgentIdentity,
    interface: &'a AgentInterface,
}

struct A2aRunEvent {
    event: RunEvent,
    run_id: Uuid,
    context_id: String,
}

struct A2aTaskInput {
    run: RunResponse,
    id: TaskId,
    artifacts: Option<Vec<Artifact>>,
}

impl FromA2aType<SendMessageRequest> for A2aRunInput {
    type Error = ApiError;

    fn from_a2a_type(request: SendMessageRequest) -> Result<Self, Self::Error> {
        let tenant_id = request.tenant.ok_or_else(|| {
            ApiError::bad_request(
                "A2A requests must include a tenant resolved by the endpoint layer",
            )
        })?;
        let context_id = request
            .message
            .context_id
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        let session_id = Uuid::parse_str(&context_id).unwrap_or_else(|_| {
            Uuid::new_v5(
                &Uuid::NAMESPACE_URL,
                context_id.as_bytes(),
            )
        });
        let run_id = request
            .message
            .task_id
            .as_ref()
            .map(|task_id| {
                Uuid::try_from(task_id)
                    .map_err(|_| ApiError::bad_request("A2A task IDs must contain UUID values"))
            })
            .transpose()?
            .unwrap_or_else(Uuid::new_v4);
        let mut user_content = Vec::new();
        let mut data_parts = Vec::new();

        for part in request.message.parts {
            match part.content {
                PartContent::Text(text) => user_content.push(UserContent::text(text)),
                PartContent::Data(data) => data_parts.push(data),
                PartContent::Raw(_) | PartContent::Url(_) => {
                    return Err(ApiError::bad_request(
                        "A2A raw and URL parts are not supported yet",
                    ));
                }
            }
        }

        if user_content.is_empty() && data_parts.is_empty() {
            return Err(ApiError::bad_request(
                "A2A messages must contain a supported part",
            ));
        }

        let context = (!data_parts.is_empty()).then(|| {
            json!({
                "a2a_input": if data_parts.len() == 1 {
                    data_parts.into_iter().next().unwrap_or(Value::Null)
                } else {
                    Value::Array(data_parts)
                }
            })
        });

        Ok(Self {
            context_id,
            run_id,
            params: RunParams::new(tenant_id, session_id)
                .with_run_id(run_id)
                .maybe_with_context(context)
                .with_messages([CreateMessageParams::User(CreateUserMessageParams {
                    id: Uuid::new_v4(),
                    content: user_content,
                    name: None,
                })]),
        })
    }
}

impl ToA2aType<AgentCard> for A2aAgentCardInput<'_> {
    type Error = ApiError;

    fn to_a2a_type(self) -> Result<AgentCard, Self::Error> {
        Ok(AgentCard {
            name: self.identity.name.clone(),
            description: self.identity.name.clone(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            supported_interfaces: vec![self.interface.clone()],
            capabilities: AgentCapabilities {
                streaming: Some(true),
                push_notifications: Some(false),
                extensions: None,
                extended_agent_card: Some(false),
            },
            default_input_modes: vec![
                "text/plain".to_string(),
                "application/json".to_string(),
            ],
            default_output_modes: vec![
                "text/plain".to_string(),
                "application/json".to_string(),
            ],
            skills: vec![AgentSkill {
                id: "react".to_string(),
                name: self.identity.name.clone(),
                description: self.identity.name.clone(),
                tags: vec!["react".to_string()],
                examples: None,
                input_modes: None,
                output_modes: None,
                security_requirements: None,
            }],
            provider: None,
            documentation_url: None,
            icon_url: None,
            security_schemes: None,
            security_requirements: None,
            signatures: None,
        })
    }
}

impl ToA2aType<Task> for A2aTaskInput {
    type Error = ApiError;

    fn to_a2a_type(self) -> Result<Task, Self::Error> {
        Ok(Task {
            id: self.id,
            context_id: self.run.session_id.to_string(),
            status: TaskStatus {
                state: match self.run.status {
                    RunStatus::Running => TaskState::Working,
                    RunStatus::Interrupted => TaskState::InputRequired,
                    RunStatus::Completed => TaskState::Completed,
                    RunStatus::Failed => TaskState::Failed,
                    RunStatus::Cancelled => TaskState::Canceled,
                },
                message: None,
                timestamp: Some(self.run.updated_at),
            },
            artifacts: self.artifacts,
            history: None,
            metadata: None,
        })
    }
}

impl ToA2aType<Artifact> for StateSnapshot {
    type Error = ApiError;

    fn to_a2a_type(self) -> Result<Artifact, Self::Error> {
        Ok(Artifact {
            artifact_id: ArtifactId::new("final-state"),
            name: Some("Final State".to_string()),
            description: None,
            parts: vec![Part::data(self.context)],
            metadata: None,
            extensions: None,
        })
    }
}

#[async_trait]
impl A2aRunCancel for ApplicationServiceA2aCancel {
    async fn cancel(&self) -> Result<(), ApiError> {
        self.service
            .cancel_run(&self.tenant_id, self.run_id)
            .await
            .map_err(ApiError::from)
            .map(|_| ())
    }
}

#[async_trait]
impl A2aService for ApplicationService {
    fn agent_card(&self, interface: &AgentInterface) -> AgentCard {
        A2aAgentCardInput {
            identity: self.agent.identity(),
            interface,
        }
        .to_a2a_type()
        .expect("A2A Agent Card conversion should not fail")
    }

    async fn send_message(
        &self,
        request: SendMessageRequest,
    ) -> Result<SendMessageResponse, ApiError> {
        let mut stream = self.stream_message(request).await?;
        let mut task = None;

        while let Some(event) = stream.next().await {
            let event = event?;

            match event {
                StreamResponse::Task(value) => task = Some(value),
                StreamResponse::StatusUpdate(update) => {
                    if let Some(value) = task.as_mut() {
                        value.status = update.status;
                    }
                }
                StreamResponse::ArtifactUpdate(update) => {
                    if let Some(value) = task.as_mut() {
                        value.artifacts.get_or_insert_default().push(update.artifact);
                    }
                }
                StreamResponse::Message(message) => {
                    return Ok(SendMessageResponse::Message(message));
                }
            }
        }

        task.map(SendMessageResponse::Task)
            .ok_or_else(|| ApiError::unexpected_error("A2A stream ended without a task"))
    }

    async fn stream_message(
        &self,
        request: SendMessageRequest,
    ) -> Result<A2aStream, ApiError> {
        let tenant_id = request.tenant.clone().ok_or_else(|| {
            ApiError::bad_request(
                "A2A requests must include a tenant resolved by the endpoint layer",
            )
        })?;
        let input = A2aRunInput::from_a2a_type(request)?;
        let run_id = input.run_id;
        let context_id = input.context_id.clone();
        let stream = self
            .run(input.params)
            .await
            .map_err(ApiError::from)?;

        Ok(
            A2aStream::new(Box::pin(async_stream::stream! {
                yield Ok(StreamResponse::Task(Task {
                    id: TaskId::new(run_id.to_string()),
                    context_id: context_id.clone(),
                    status: TaskStatus {
                        state: TaskState::Submitted,
                        message: None,
                        timestamp: Some(Utc::now()),
                    },
                    artifacts: None,
                    history: None,
                    metadata: None,
                }));

                let mut events = stream;

                while let Some(event) = events.next().await {
                    match (A2aRunEvent {
                        event,
                        run_id,
                        context_id: context_id.clone(),
                    })
                    .to_a2a_type()
                    {
                        Ok(events) => {
                            for event in events {
                                yield Ok(event);
                            }
                        }
                        Err(error) => yield Err(error),
                    }
                }
            }))
            .with_cancel(ApplicationServiceA2aCancel {
                service: self.clone(),
                tenant_id,
                run_id,
            }),
        )
    }

    async fn get_task(
        &self,
        request: GetTaskRequest,
    ) -> Result<Task, ApiError> {
        let run_id = Uuid::try_from(&request.id)
            .map_err(|_| ApiError::bad_request("A2A task IDs must contain UUID values"))?;

        let tenant_id = request.tenant.ok_or_else(|| {
            ApiError::bad_request(
                "A2A requests must include a tenant",
            )
        })?;

        let run = self
            .get_run(&tenant_id, run_id)
            .await
            .map_err(ApiError::from)?;

        let snapshot = self.scope_factory
            .ro_scope(|scope| {
                Box::pin(async move {
                    scope
                        .state_snapshot_repo()
                        .find(
                            FindStateSnapshotParams::new()
                                .no_limit()
                                .tenant_ids([tenant_id.clone()])
                                .run_ids([run_id]),
                        )
                        .await
                        .map_err(|err| SqlScopeFactoryError::source_unexpected(err))
                })
            })
            .await
            .map_err(|error| ApiError::unexpected_error(error.to_string()))?
            .into_iter()
            .next();

        A2aTaskInput {
            run,
            id: request.id,
            artifacts: snapshot
                .map(|snapshot| {
                    snapshot
                        .to_a2a_type()
                        .map(|artifact| vec![artifact])
                })
                .transpose()?,
        }
        .to_a2a_type()
    }

    async fn cancel_task(
        &self,
        request: CancelTaskRequest,
    ) -> Result<Task, ApiError> {
        let run_id = Uuid::try_from(&request.id)
            .map_err(|_| ApiError::bad_request("A2A task IDs must contain UUID values"))?;
        let tenant_id = request.tenant.clone().ok_or_else(|| {
            ApiError::bad_request(
                "A2A requests must include a tenant resolved by the endpoint layer",
            )
        })?;

        self.cancel_run(&tenant_id, run_id)
            .await
            .map_err(ApiError::from)?;

        self.get_task(GetTaskRequest {
            id: request.id,
            history_length: None,
            tenant: request.tenant,
        })
        .await
    }
}

impl ToA2aType<Vec<StreamResponse>> for A2aRunEvent {
    type Error = ApiError;

    fn to_a2a_type(self) -> Result<Vec<StreamResponse>, Self::Error> {
        let task_id = TaskId::new(self.run_id.to_string());

        Ok(match self.event {
            RunEvent::RunStarted { .. } => {
                vec![StreamResponse::StatusUpdate(TaskStatusUpdateEvent {
                    task_id,
                    context_id: self.context_id,
                    status: TaskStatus {
                        state: TaskState::Working,
                        message: None,
                        timestamp: Some(Utc::now()),
                    },
                    metadata: None,
                })]
            }
            RunEvent::TextMessageContent { delta, .. } => {
                vec![StreamResponse::ArtifactUpdate(TaskArtifactUpdateEvent {
                    task_id,
                    context_id: self.context_id,
                    artifact: Artifact {
                        artifact_id: ArtifactId::new("response"),
                        name: Some("Agent Response".to_string()),
                        description: None,
                        parts: vec![Part::text(delta)],
                        metadata: None,
                        extensions: None,
                    },
                    append: Some(true),
                    last_chunk: Some(false),
                    metadata: None,
                })]
            }
            RunEvent::RunFinished { status, result, .. } => {
                let mut events = Vec::new();

                if let Some(result) = result {
                    events.push(StreamResponse::ArtifactUpdate(TaskArtifactUpdateEvent {
                        task_id: task_id.clone(),
                        context_id: self.context_id.clone(),
                        artifact: Artifact {
                            artifact_id: ArtifactId::new("state"),
                            name: Some("Final State".to_string()),
                            description: None,
                            parts: vec![Part::data(result.context)],
                            metadata: None,
                            extensions: None,
                        },
                        append: Some(false),
                        last_chunk: Some(true),
                        metadata: None,
                    }));
                }

                events.push(StreamResponse::StatusUpdate(TaskStatusUpdateEvent {
                    task_id,
                    context_id: self.context_id,
                    status: TaskStatus {
                        state: match status {
                            RunStatus::Running => TaskState::Working,
                            RunStatus::Interrupted => TaskState::InputRequired,
                            RunStatus::Completed => TaskState::Completed,
                            RunStatus::Failed => TaskState::Failed,
                            RunStatus::Cancelled => TaskState::Canceled,
                        },
                        message: None,
                        timestamp: Some(Utc::now()),
                    },
                    metadata: None,
                }));

                events
            }
            RunEvent::RunError { error, .. } => {
                vec![StreamResponse::StatusUpdate(TaskStatusUpdateEvent {
                    task_id,
                    context_id: self.context_id,
                    status: TaskStatus {
                        state: TaskState::Failed,
                        message: Some(Message::new(Role::Agent, vec![Part::text(error)])),
                        timestamp: Some(Utc::now()),
                    },
                    metadata: None,
                })]
            }
            _ => Vec::new(),
        })
    }
}
