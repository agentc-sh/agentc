// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use futures::stream::{StreamExt, TryStreamExt};
use json_patch::Patch;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, from_value, json, to_string};
use std::{
    convert::Infallible,
    fmt::{Debug, Display, Formatter, Result as FmtResult},
    ops::Deref,
    str::FromStr,
};
use url::Url;
use utoipa::ToSchema;
use uuid::Uuid;

use agentc_agent::types::tools::ToolDefinition;
use agentc_domain::types::run::RunStatus;
use agentc_http::server::errors::ApiError;
use agentc_protocol_ag_ui::{
    protocol::{
        event::{
            BaseEvent, CustomEvent, Event, MessagesSnapshotEvent, ReasoningEncryptedValueEvent,
            ReasoningEncryptedValueSubtype, ReasoningEndEvent, ReasoningMessageContentEvent,
            ReasoningMessageEndEvent, ReasoningMessageStartEvent, ReasoningStartEvent,
            RunErrorEvent, RunFinishedEvent, RunStartedEvent, StateDeltaEvent, StateSnapshotEvent,
            TextMessageContentEvent, TextMessageEndEvent, TextMessageStartEvent, Timestamp,
            ToolCallArgsEvent, ToolCallEndEvent, ToolCallResultEvent, ToolCallStartEvent,
        },
        ids::{MessageId, RunId, ThreadId},
        input::RunAgentInput,
        message::{
            InputContent, InputContentDataSource, InputContentSource, InputContentUrlSource,
            Message, Role, UserMessageContent,
        },
        outcome::{Interrupt, ResumeEntry, ResumeStatus, RunFinishedOutcome},
        tool::{FunctionCall, ToolCall},
    },
    traits::{AgUiRunCancel, AgUiRunStream, AgUiService, FromAgUiType, ToAgUiType},
};

use crate::{
    service::{
        ApplicationService,
        errors::ServiceError,
        operations::run::RunOperations,
        types::{
            message::{
                CreateMessageParams, CreateSystemMessageParams, CreateToolMessageParams,
                CreateUserMessageParams, MessageResponse,
            },
            run::{RunEvent, RunParams},
        },
    },
    types::{
        context_var::ContextVar,
        event::ReasoningSignatureSubtype,
        message::{
            Audio as DomainAudio, Document as DomainDocument, Image as DomainImage,
            MediaSource as DomainMediaSource, UserContent as DomainUserContent,
            Video as DomainVideo,
        },
    },
};

const AG_UI_INTERRUPT_REASON: &str = "interrupt";

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord, ToSchema)]
pub struct DeterministicUuid(Uuid);

impl DeterministicUuid {
    const NAMESPACE: Uuid = Uuid::NAMESPACE_DNS;

    pub fn new(value: Uuid) -> Self {
        Self(value)
    }

    pub fn new_v4() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn new_v5(value: &str) -> Self {
        Self(Uuid::new_v5(&Self::NAMESPACE, value.as_bytes()))
    }

    pub fn as_inner(&self) -> Uuid {
        self.0
    }

    pub fn as_inner_mut(&mut self) -> &mut Uuid {
        &mut self.0
    }

    pub fn into_inner(self) -> Uuid {
        self.0
    }
}

impl From<&str> for DeterministicUuid {
    fn from(value: &str) -> Self {
        Uuid::parse_str(value).map_or_else(|_| Self::new_v5(value), Self::new)
    }
}

impl FromStr for DeterministicUuid {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::from(s))
    }
}

impl Display for DeterministicUuid {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        Display::fmt(&self.as_inner(), f)
    }
}

impl Debug for DeterministicUuid {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        Debug::fmt(&self.as_inner(), f)
    }
}

impl From<Uuid> for DeterministicUuid {
    fn from(value: Uuid) -> Self {
        Self(value)
    }
}

impl From<DeterministicUuid> for Uuid {
    fn from(value: DeterministicUuid) -> Self {
        value.into_inner()
    }
}

impl Deref for DeterministicUuid {
    type Target = Uuid;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<DeterministicUuid> for ThreadId {
    fn from(value: DeterministicUuid) -> Self {
        value.into_inner().into()
    }
}

impl From<ThreadId> for DeterministicUuid {
    fn from(value: ThreadId) -> Self {
        Self::from(&*value)
    }
}

impl From<DeterministicUuid> for RunId {
    fn from(value: DeterministicUuid) -> Self {
        value.into_inner().into()
    }
}

impl From<RunId> for DeterministicUuid {
    fn from(value: RunId) -> Self {
        Self::from(&*value)
    }
}

impl From<MessageId> for DeterministicUuid {
    fn from(value: MessageId) -> Self {
        Self::from(&*value)
    }
}

impl From<DeterministicUuid> for MessageId {
    fn from(value: DeterministicUuid) -> Self {
        value.into_inner().into()
    }
}

impl ToAgUiType<InputContent> for DomainUserContent {
    type Error = ServiceError;

    fn to_ag_ui_type(self) -> Result<InputContent, Self::Error> {
        Ok(match self {
            DomainUserContent::Text(text) => InputContent::Text { text },
            DomainUserContent::Image(img) => InputContent::Image {
                source: match img.source {
                    DomainMediaSource::Url(url) => InputContentSource::Url(InputContentUrlSource {
                        value: url.to_string(),
                        mime_type: Some(img.media_type),
                    }),
                    DomainMediaSource::Base64(data) => {
                        InputContentSource::Data(InputContentDataSource {
                            value: data,
                            mime_type: img.media_type,
                        })
                    }
                },
                metadata: None,
            },
            DomainUserContent::Audio(audio) => InputContent::Audio {
                source: match audio.source {
                    DomainMediaSource::Url(url) => InputContentSource::Url(InputContentUrlSource {
                        value: url.to_string(),
                        mime_type: Some(audio.media_type),
                    }),
                    DomainMediaSource::Base64(data) => {
                        InputContentSource::Data(InputContentDataSource {
                            value: data,
                            mime_type: audio.media_type,
                        })
                    }
                },
                metadata: None,
            },
            DomainUserContent::Video(video) => InputContent::Video {
                source: match video.source {
                    DomainMediaSource::Url(url) => InputContentSource::Url(InputContentUrlSource {
                        value: url.to_string(),
                        mime_type: Some(video.media_type),
                    }),
                    DomainMediaSource::Base64(data) => {
                        InputContentSource::Data(InputContentDataSource {
                            value: data,
                            mime_type: video.media_type,
                        })
                    }
                },
                metadata: None,
            },
            DomainUserContent::Document(doc) => InputContent::Document {
                source: match doc.source {
                    DomainMediaSource::Url(url) => InputContentSource::Url(InputContentUrlSource {
                        value: url.to_string(),
                        mime_type: Some(doc.media_type),
                    }),
                    DomainMediaSource::Base64(data) => {
                        InputContentSource::Data(InputContentDataSource {
                            value: data,
                            mime_type: doc.media_type,
                        })
                    }
                },
                metadata: None,
            },
        })
    }
}

impl FromAgUiType<InputContent> for DomainUserContent {
    type Error = ServiceError;

    fn from_ag_ui_type(value: InputContent) -> Result<Self, Self::Error> {
        Ok(match value {
            InputContent::Text { text } => DomainUserContent::Text(text),
            InputContent::Image { source, .. } => DomainUserContent::Image(match source {
                InputContentSource::Url(s) => DomainImage {
                    source: DomainMediaSource::Url(
                        Url::parse(&s.value)
                            .map_err(|err| ServiceError::invalid_input(err.to_string()))?,
                    ),
                    media_type: s.mime_type.ok_or_else(|| {
                        ServiceError::invalid_input("mimeType is required for url sources")
                    })?,
                },
                InputContentSource::Data(s) => DomainImage {
                    source: DomainMediaSource::Base64(s.value),
                    media_type: s.mime_type,
                },
            }),
            InputContent::Audio { source, .. } => DomainUserContent::Audio(match source {
                InputContentSource::Url(s) => DomainAudio {
                    source: DomainMediaSource::Url(
                        Url::parse(&s.value)
                            .map_err(|err| ServiceError::invalid_input(err.to_string()))?,
                    ),
                    media_type: s.mime_type.ok_or_else(|| {
                        ServiceError::invalid_input("mimeType is required for url sources")
                    })?,
                },
                InputContentSource::Data(s) => DomainAudio {
                    source: DomainMediaSource::Base64(s.value),
                    media_type: s.mime_type,
                },
            }),
            InputContent::Video { source, .. } => DomainUserContent::Video(match source {
                InputContentSource::Url(s) => DomainVideo {
                    source: DomainMediaSource::Url(
                        Url::parse(&s.value)
                            .map_err(|err| ServiceError::invalid_input(err.to_string()))?,
                    ),
                    media_type: s.mime_type.ok_or_else(|| {
                        ServiceError::invalid_input("mimeType is required for url sources")
                    })?,
                },
                InputContentSource::Data(s) => DomainVideo {
                    source: DomainMediaSource::Base64(s.value),
                    media_type: s.mime_type,
                },
            }),
            InputContent::Document { source, .. } => DomainUserContent::Document(match source {
                InputContentSource::Url(s) => DomainDocument {
                    source: DomainMediaSource::Url(
                        Url::parse(&s.value)
                            .map_err(|err| ServiceError::invalid_input(err.to_string()))?,
                    ),
                    media_type: s.mime_type.ok_or_else(|| {
                        ServiceError::invalid_input("mimeType is required for url sources")
                    })?,
                },
                InputContentSource::Data(s) => DomainDocument {
                    source: DomainMediaSource::Base64(s.value),
                    media_type: s.mime_type,
                },
            }),
        })
    }
}

impl ToAgUiType<Event> for RunEvent {
    type Error = ServiceError;

    #[allow(unreachable_patterns)]
    fn to_ag_ui_type(self) -> Result<Event, Self::Error> {
        match self {
            Self::RunStarted { timestamp, session_id, run_id } => {
                Ok(Event::RunStarted(RunStartedEvent {
                    base: BaseEvent::at(Timestamp::from_secs_f64(timestamp)),
                    thread_id: session_id.into(),
                    run_id: run_id.into(),
                }))
            }
            Self::RunFinished {
                timestamp,
                session_id,
                run_id,
                status,
                interrupt_payload,
                result,
            } => Ok(Event::RunFinished(RunFinishedEvent {
                base: BaseEvent::at(Timestamp::from_secs_f64(timestamp)),
                thread_id: session_id.into(),
                run_id: run_id.into(),
                result: result
                    .map(|state| state.context)
                    .filter(|context| !context.is_null()),
                outcome: match status {
                    RunStatus::Completed => Some(RunFinishedOutcome::Success),
                    RunStatus::Cancelled => Some(RunFinishedOutcome::Cancelled),
                    RunStatus::Interrupted => Some(RunFinishedOutcome::Interrupt {
                        interrupts: vec![Interrupt {
                            id: run_id.into(),
                            reason: AG_UI_INTERRUPT_REASON.to_string(),
                            metadata: interrupt_payload
                                .map(|payload| Map::from_iter([("payload".to_string(), payload)])),
                        }],
                    }),
                    RunStatus::Running | RunStatus::Failed => None,
                },
            })),
            Self::RunError { timestamp, error, code, .. } => Ok(Event::RunError(RunErrorEvent {
                base: BaseEvent::at(Timestamp::from_secs_f64(timestamp)),
                message: error,
                code,
            })),
            Self::TextMessageStart { timestamp, message_id } => {
                Ok(Event::TextMessageStart(TextMessageStartEvent {
                    base: BaseEvent::at(Timestamp::from_secs_f64(timestamp)),
                    message_id: message_id.into(),
                    role: Role::Assistant, // Text messages from the agent are always assistant messages
                }))
            }
            Self::TextMessageContent { timestamp, message_id, delta } => {
                Ok(Event::TextMessageContent(TextMessageContentEvent {
                    base: BaseEvent::at(Timestamp::from_secs_f64(timestamp)),
                    message_id: message_id.into(),
                    delta,
                }))
            }
            Self::TextMessageEnd { timestamp, message_id } => {
                Ok(Event::TextMessageEnd(TextMessageEndEvent {
                    base: BaseEvent::at(Timestamp::from_secs_f64(timestamp)),
                    message_id: message_id.into(),
                }))
            }
            Self::ToolCallStart { timestamp, tool_call_id, tool_name } => {
                Ok(Event::ToolCallStart(ToolCallStartEvent {
                    base: BaseEvent::at(Timestamp::from_secs_f64(timestamp)),
                    tool_call_id: tool_call_id.into(),
                    tool_call_name: tool_name,
                    parent_message_id: None,
                }))
            }
            Self::ToolCallArgs { timestamp, tool_call_id, delta } => {
                Ok(Event::ToolCallArgs(ToolCallArgsEvent {
                    base: BaseEvent::at(Timestamp::from_secs_f64(timestamp)),
                    tool_call_id: tool_call_id.into(),
                    delta,
                }))
            }
            Self::ToolCallEnd { timestamp, tool_call_id } => {
                Ok(Event::ToolCallEnd(ToolCallEndEvent {
                    base: BaseEvent::at(Timestamp::from_secs_f64(timestamp)),
                    tool_call_id: tool_call_id.into(),
                }))
            }
            Self::ToolCallResult {
                timestamp,
                tool_call_id,
                message_id,
                content,
            } => Ok(Event::ToolCallResult(ToolCallResultEvent {
                base: BaseEvent::at(Timestamp::from_secs_f64(timestamp)),
                tool_call_id: tool_call_id.into(),
                message_id: message_id.into(),
                content: match content {
                    Value::String(text) => text,
                    other => other.to_string(),
                },
                role: Role::Tool,
            })),
            Self::ToolCallError {
                timestamp,
                error,
                tool_call_id,
                message_id,
                ..
            } => Ok(Event::ToolCallResult(ToolCallResultEvent {
                base: BaseEvent::at(Timestamp::from_secs_f64(timestamp)),
                tool_call_id: tool_call_id.into(),
                message_id: message_id.into(),
                content: error,
                role: Role::Tool,
            })),
            Self::ReasoningStart { timestamp, message_id } => {
                Ok(Event::ReasoningStart(ReasoningStartEvent {
                    base: BaseEvent::at(Timestamp::from_secs_f64(timestamp)),
                    message_id: message_id.into(),
                }))
            }
            Self::ReasoningEnd { timestamp, message_id } => {
                Ok(Event::ReasoningEnd(ReasoningEndEvent {
                    base: BaseEvent::at(Timestamp::from_secs_f64(timestamp)),
                    message_id: message_id.into(),
                }))
            }
            Self::ReasoningMessageStart { timestamp, message_id } => {
                Ok(Event::ReasoningMessageStart(ReasoningMessageStartEvent {
                    base: BaseEvent::at(Timestamp::from_secs_f64(timestamp)),
                    message_id: message_id.into(),
                    role: Role::Reasoning,
                }))
            }
            Self::ReasoningMessageContent { timestamp, message_id, delta } => {
                Ok(Event::ReasoningMessageContent(ReasoningMessageContentEvent {
                    base: BaseEvent::at(Timestamp::from_secs_f64(timestamp)),
                    message_id: message_id.into(),
                    delta,
                }))
            }
            Self::ReasoningMessageEnd { timestamp, message_id } => {
                Ok(Event::ReasoningMessageEnd(ReasoningMessageEndEvent {
                    base: BaseEvent::at(Timestamp::from_secs_f64(timestamp)),
                    message_id: message_id.into(),
                }))
            }
            Self::ReasoningSignature { timestamp, subtype, entity_id, value, .. } => {
                Ok(Event::ReasoningEncryptedValue(ReasoningEncryptedValueEvent {
                    base: BaseEvent::at(Timestamp::from_secs_f64(timestamp)),
                    subtype: match subtype {
                        ReasoningSignatureSubtype::Message => {
                            ReasoningEncryptedValueSubtype::Message
                        }
                        ReasoningSignatureSubtype::ToolCall => {
                            ReasoningEncryptedValueSubtype::ToolCall
                        }
                    },
                    entity_id,
                    encrypted_value: value,
                }))
            }
            Self::StateSnapshot { timestamp, state } => {
                Ok(Event::StateSnapshot(StateSnapshotEvent {
                    base: BaseEvent::at(Timestamp::from_secs_f64(timestamp)),
                    snapshot: state.context,
                }))
            }
            Self::StateDelta { timestamp, delta } => Ok(Event::StateDelta(StateDeltaEvent {
                base: BaseEvent::at(Timestamp::from_secs_f64(timestamp)),
                delta: Patch(delta.context),
            })),
            Self::MessagesSnapshot { timestamp, messages } => {
                Ok(Event::MessagesSnapshot(MessagesSnapshotEvent {
                    base: BaseEvent::at(Timestamp::from_secs_f64(timestamp)),
                    messages: messages
                        .into_iter()
                        .map(ToAgUiType::to_ag_ui_type)
                        .collect::<Result<Vec<_>, ServiceError>>()
                        .map_err(|_| {
                            ServiceError::unexpected(
                                "Failed to convert messages in messages snapshot",
                            )
                        })?,
                }))
            }
            Self::ActivityDelta {
                timestamp,
                tool_call_id,
                activity_type,
                patch,
            } => Ok(Event::Custom(CustomEvent {
                base: BaseEvent::at(Timestamp::from_secs_f64(timestamp)),
                name: "ACTIVITY_DELTA".to_string(),
                value: json!({
                    "tool_call_id": tool_call_id,
                    "activity_type": activity_type,
                    "patch": patch,
                }),
            })),
            _ => Err(ServiceError::unexpected("Unsupported event type for AG-UI protocol")),
        }
    }
}

impl ToAgUiType<Message> for MessageResponse {
    type Error = ServiceError;

    #[allow(unreachable_patterns)]
    fn to_ag_ui_type(self) -> Result<Message, Self::Error> {
        match self {
            Self::System(response) => Ok(Message::System {
                id: response.id.into(),
                content: response.content,
                name: response.name,
            }),
            Self::User(response) => Ok(Message::User {
                id: response.id.into(),
                content: UserMessageContent::Parts(
                    response
                        .content
                        .into_iter()
                        .map(ToAgUiType::to_ag_ui_type)
                        .collect::<Result<_, _>>()?,
                ),
                name: response.name,
            }),
            Self::Assistant(response) => Ok(Message::Assistant {
                id: response.id.into(),
                content: response.content,
                name: response.name,
                tool_calls: response
                    .tool_calls
                    .map(|calls| {
                        calls
                            .into_iter()
                            .map(|call| {
                                Ok(ToolCall {
                                    id: call.id.into(),
                                    call_type: "function".to_string(),
                                    function: FunctionCall {
                                        name: call.name,
                                        arguments: to_string(&call.arguments).map_err(|_| {
                                            ServiceError::unexpected(
                                                "Failed to serialize tool call arguments",
                                            )
                                        })?,
                                    },
                                })
                            })
                            .collect::<Result<Vec<_>, ServiceError>>()
                    })
                    .transpose()?,
            }),
            Self::Tool(response) => Ok(Message::Tool {
                id: response.id.into(),
                content: response
                    .error
                    .clone()
                    .unwrap_or(response.content.unwrap_or_default()),
                tool_call_id: response.tool_call_id.into(),
                error: response.error,
            }),
            Self::Reasoning(response) => Ok(Message::Reasoning {
                id: response.id.into(),
                content: response.content,
                encrypted_value: response.signature,
            }),
            _ => Err(ServiceError::unexpected("Unsupported message type in response"))?,
        }
    }
}

impl<'a> FromAgUiType<(RunAgentInput, &'a str)> for RunParams {
    type Error = ServiceError;

    fn from_ag_ui_type((input, tenant_id): (RunAgentInput, &'a str)) -> Result<Self, Self::Error> {
        let mut entries = input.resume.into_iter();

        Ok(RunParams::new(tenant_id, DeterministicUuid::from(input.thread_id))
            .with_run_id(DeterministicUuid::from(input.run_id))
            .maybe_with_resume_payload(match (entries.next(), entries.next()) {
                (None, _) => None,
                (
                    Some(ResumeEntry {
                        status: ResumeStatus::Resolved, payload, ..
                    }),
                    None,
                ) => Some(payload.unwrap_or(Value::Null)),
                (Some(ResumeEntry { status: ResumeStatus::Cancelled, .. }), None) => {
                    return Err(ServiceError::invalid_input(
                        "cancelling an interrupt is not supported",
                    ));
                }
                (Some(_), Some(_)) => {
                    return Err(ServiceError::invalid_input("a run resumes at most one interrupt"));
                }
            })
            .maybe_with_model(
                input
                    .forwarded_props
                    .as_object()
                    .and_then(|props| props.get("model"))
                    .and_then(|value| from_value(value.clone()).ok()),
            )
            .maybe_with_capability_override(
                input
                    .forwarded_props
                    .as_object()
                    .and_then(|props| props.get("capability_override"))
                    .and_then(|value| from_value(value.clone()).ok()),
            )
            .with_context_vars(
                input
                    .context
                    .into_iter()
                    .map(|context| ContextVar {
                        description: context.description,
                        value: context.value,
                    }),
            )
            .with_tools(input.tools.into_iter().map(|tool| {
                ToolDefinition {
                    name: tool.name,
                    description: tool.description,
                    parameters: tool
                        .parameters
                        .unwrap_or_else(|| json!({ "type": "object", "properties": {} })),
                }
            }))
            .with_messages(
                input
                    .messages
                    .into_iter()
                    .map(|message| {
                        Ok(match message {
                            Message::System { id, content, name } => {
                                Some(CreateMessageParams::System(CreateSystemMessageParams {
                                    id: DeterministicUuid::from(id).into(),
                                    content,
                                    name,
                                }))
                            }
                            Message::User { id, content, name } => {
                                Some(CreateMessageParams::User(CreateUserMessageParams {
                                    id: DeterministicUuid::from(id).into(),
                                    name,
                                    content: match content {
                                        UserMessageContent::Text(text) => {
                                            vec![DomainUserContent::Text(text)]
                                        }
                                        UserMessageContent::Parts(parts) => parts
                                            .into_iter()
                                            .map(DomainUserContent::from_ag_ui_type)
                                            .collect::<Result<_, _>>()?,
                                    },
                                }))
                            }
                            Message::Tool { id, content, tool_call_id, error } => {
                                Some(CreateMessageParams::Tool(CreateToolMessageParams {
                                    id: DeterministicUuid::from(id).into(),
                                    content: Some(content),
                                    tool_call_id: tool_call_id.into(),
                                    parent_message_id: None,
                                    error,
                                    name: None,
                                }))
                            }
                            _ => None,
                        })
                    })
                    .collect::<Result<Vec<_>, ServiceError>>()?
                    .into_iter()
                    .flatten(),
            ))
    }
}

struct ApplicationServiceAgUiCancel {
    service: ApplicationService,
    tenant_id: String,
    run_id: Uuid,
}

#[async_trait]
impl AgUiRunCancel for ApplicationServiceAgUiCancel {
    async fn cancel(&self) -> Result<(), ApiError> {
        self.service
            .cancel_run(&self.tenant_id, self.run_id)
            .await?;

        Ok(())
    }
}

#[async_trait]
impl AgUiService for ApplicationService {
    async fn ag_ui_run(
        &self,
        input: RunAgentInput,
        tenant_id: &str,
    ) -> Result<AgUiRunStream, ApiError> {
        let run_id = DeterministicUuid::from(&*input.run_id);
        let tenant_id = tenant_id.to_string();
        let stream = self
            .run(RunParams::from_ag_ui_type((input, tenant_id.as_str()))?)
            .await?;

        Ok(AgUiRunStream::new(Box::pin(
            stream
                .map(|event| event.to_ag_ui_type())
                .map_err(Into::into),
        ))
        .with_cancel(ApplicationServiceAgUiCancel {
            service: self.clone(),
            tenant_id,
            run_id: run_id.into(),
        }))
    }
}
