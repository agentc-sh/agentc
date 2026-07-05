// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use futures::stream::{BoxStream, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{from_value, json, to_string, to_value};
use std::{
    fmt::{Debug, Display, Formatter, Result as FmtResult},
    ops::Deref,
    str::FromStr,
};
use url::Url;
use utoipa::ToSchema;
use uuid::Uuid;

use agentc_agent::types::tools::ToolDefinition;
use agentc_protocol_ag_ui::{
    protocol::{
        event::{
            BaseEvent, CustomEvent, Event, MessagesSnapshotEvent, ReasoningEncryptedValueEvent,
            ReasoningEncryptedValueSubtype, ReasoningEndEvent, ReasoningMessageContentEvent,
            ReasoningMessageEndEvent, ReasoningMessageStartEvent, ReasoningStartEvent,
            RunErrorEvent, RunFinishedEvent, RunStartedEvent, StateDeltaEvent, StateSnapshotEvent,
            TextMessageContentEvent, TextMessageEndEvent, TextMessageStartEvent, ToolCallArgsEvent,
            ToolCallEndEvent, ToolCallResultEvent, ToolCallStartEvent,
        },
        ids::{RunId, ThreadId},
        input::RunAgentInput,
        message::{
            InputContent, InputContentDataSource, InputContentSource, InputContentUrlSource,
            Message, Role, UserMessageContent,
        },
        tool::{FunctionCall, ToolCall},
    },
    traits::{AgUiService, FromAgUiType, ToAgUiType},
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
                            .map_err(|e| ServiceError::unexpected(e.to_string()))?,
                    ),
                    media_type: s.mime_type.unwrap_or_default(),
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
                            .map_err(|e| ServiceError::unexpected(e.to_string()))?,
                    ),
                    media_type: s.mime_type.unwrap_or_default(),
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
                            .map_err(|e| ServiceError::unexpected(e.to_string()))?,
                    ),
                    media_type: s.mime_type.unwrap_or_default(),
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
                            .map_err(|e| ServiceError::unexpected(e.to_string()))?,
                    ),
                    media_type: s.mime_type.unwrap_or_default(),
                },
                InputContentSource::Data(s) => DomainDocument {
                    source: DomainMediaSource::Base64(s.value),
                    media_type: s.mime_type,
                },
            }),
        })
    }
}

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

    pub fn try_parse_str(value: &str) -> Result<Self, uuid::Error> {
        if let Ok(uuid) = Uuid::parse_str(value) {
            Ok(Self::new(uuid))
        } else {
            Ok(Self::new_v5(value))
        }
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

impl FromStr for DeterministicUuid {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_parse_str(s)
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
        Self::new(value.into())
    }
}

impl From<DeterministicUuid> for RunId {
    fn from(value: DeterministicUuid) -> Self {
        value.into_inner().into()
    }
}

impl From<RunId> for DeterministicUuid {
    fn from(value: RunId) -> Self {
        Self::new(value.into())
    }
}

impl ToAgUiType<Event> for RunEvent {
    type Error = ServiceError;

    #[allow(unreachable_patterns)]
    fn to_ag_ui_type(self) -> Result<Event, Self::Error> {
        match self {
            Self::RunStarted { timestamp, session_id, run_id } => {
                Ok(Event::RunStarted(RunStartedEvent {
                    base: BaseEvent {
                        timestamp: Some(timestamp),
                        raw_event: None,
                    },
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
                base: BaseEvent {
                    timestamp: Some(timestamp),
                    raw_event: None,
                },
                thread_id: session_id.into(),
                run_id: run_id.into(),
                result: Some(json!({
                    "status": status,
                    "interrupt_payload": interrupt_payload,
                    // "state": result.map(|res| to_value(res).unwrap_or(Value::Null))
                    "state": result.map(|res| res.context),
                })),
            })),
            Self::RunError { timestamp, error, code, .. } => Ok(Event::RunError(RunErrorEvent {
                base: BaseEvent {
                    timestamp: Some(timestamp),
                    raw_event: None,
                },
                message: error,
                code,
            })),
            Self::TextMessageStart { timestamp, message_id } => {
                Ok(Event::TextMessageStart(TextMessageStartEvent {
                    base: BaseEvent {
                        timestamp: Some(timestamp),
                        raw_event: None,
                    },
                    message_id: message_id.into(),
                    role: Role::Assistant, // Text messages from the agent are always assistant messages
                }))
            }
            Self::TextMessageContent { timestamp, message_id, delta } => {
                Ok(Event::TextMessageContent(TextMessageContentEvent {
                    base: BaseEvent {
                        timestamp: Some(timestamp),
                        raw_event: None,
                    },
                    message_id: message_id.into(),
                    delta,
                }))
            }
            Self::TextMessageEnd { timestamp, message_id } => {
                Ok(Event::TextMessageEnd(TextMessageEndEvent {
                    base: BaseEvent {
                        timestamp: Some(timestamp),
                        raw_event: None,
                    },
                    message_id: message_id.into(),
                }))
            }
            Self::ToolCallStart { timestamp, tool_call_id, tool_name } => {
                Ok(Event::ToolCallStart(ToolCallStartEvent {
                    base: BaseEvent {
                        timestamp: Some(timestamp),
                        raw_event: None,
                    },
                    tool_call_id: tool_call_id.into(),
                    tool_call_name: tool_name,
                    parent_message_id: None,
                }))
            }
            Self::ToolCallArgs { timestamp, tool_call_id, delta } => {
                Ok(Event::ToolCallArgs(ToolCallArgsEvent {
                    base: BaseEvent {
                        timestamp: Some(timestamp),
                        raw_event: None,
                    },
                    tool_call_id: tool_call_id.into(),
                    delta,
                }))
            }
            Self::ToolCallEnd { timestamp, tool_call_id } => {
                Ok(Event::ToolCallEnd(ToolCallEndEvent {
                    base: BaseEvent {
                        timestamp: Some(timestamp),
                        raw_event: None,
                    },
                    tool_call_id: tool_call_id.into(),
                }))
            }
            Self::ToolCallResult {
                timestamp,
                tool_call_id,
                message_id,
                content,
            } => Ok(Event::ToolCallResult(ToolCallResultEvent {
                base: BaseEvent {
                    timestamp: Some(timestamp),
                    raw_event: None,
                },
                tool_call_id: tool_call_id.into(),
                message_id: message_id.into(),
                content: to_string(&content).map_err(|_| {
                    ServiceError::unexpected("Failed to serialize tool call result content")
                })?,
                role: Role::Tool, // Tool call results are always tool messages
            })),
            Self::ToolCallError {
                timestamp,
                error,
                tool_call_id,
                message_id,
                ..
            } => Ok(Event::ToolCallResult(ToolCallResultEvent {
                base: BaseEvent {
                    timestamp: Some(timestamp),
                    raw_event: None,
                },
                tool_call_id: tool_call_id.into(),
                message_id: message_id.into(),
                content: error,
                role: Role::Tool, // Tool call results are always tool messages, even if they contain errors
            })),
            Self::ReasoningStart { timestamp, message_id } => {
                Ok(Event::ReasoningStart(ReasoningStartEvent {
                    base: BaseEvent {
                        timestamp: Some(timestamp),
                        raw_event: None,
                    },
                    message_id: message_id.into(),
                }))
            }
            Self::ReasoningEnd { timestamp, message_id } => {
                Ok(Event::ReasoningEnd(ReasoningEndEvent {
                    base: BaseEvent {
                        timestamp: Some(timestamp),
                        raw_event: None,
                    },
                    message_id: message_id.into(),
                }))
            }
            Self::ReasoningMessageStart { timestamp, message_id } => {
                Ok(Event::ReasoningMessageStart(ReasoningMessageStartEvent {
                    base: BaseEvent {
                        timestamp: Some(timestamp),
                        raw_event: None,
                    },
                    message_id: message_id.into(),
                    role: Role::Reasoning,
                }))
            }
            Self::ReasoningMessageContent { timestamp, message_id, delta } => {
                Ok(Event::ReasoningMessageContent(ReasoningMessageContentEvent {
                    base: BaseEvent {
                        timestamp: Some(timestamp),
                        raw_event: None,
                    },
                    message_id: message_id.into(),
                    delta,
                }))
            }
            Self::ReasoningMessageEnd { timestamp, message_id } => {
                Ok(Event::ReasoningMessageEnd(ReasoningMessageEndEvent {
                    base: BaseEvent {
                        timestamp: Some(timestamp),
                        raw_event: None,
                    },
                    message_id: message_id.into(),
                }))
            }
            Self::ReasoningSignature { timestamp, subtype, entity_id, value, .. } => {
                Ok(Event::ReasoningEncryptedValue(ReasoningEncryptedValueEvent {
                    base: BaseEvent {
                        timestamp: Some(timestamp),
                        raw_event: None,
                    },
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
                    base: BaseEvent {
                        timestamp: Some(timestamp),
                        raw_event: None,
                    },
                    // snapshot: to_value(state)
                    //     .map_err(|_| ServiceError::unexpected("Failed to serialize state snapshot"))?,
                    snapshot: state.context,
                }))
            }
            Self::StateDelta { timestamp, delta } => Ok(Event::StateDelta(StateDeltaEvent {
                base: BaseEvent {
                    timestamp: Some(timestamp),
                    raw_event: None,
                },
                // delta: vec![
                //     to_value(PatchOperation::Add(AddOperation {
                //         path: "/messages".try_into().expect("invalid patch path"),
                //         value: to_value(delta.messages)
                //             .map_err(|_| ServiceError::unexpected("Failed to serialize state delta messages"))?,
                //     }))
                //     .map_err(|_| ServiceError::unexpected("Failed to serialize patch operation"))?
                // ]
                delta: delta
                    .context
                    .into_iter()
                    .filter_map(|operation| to_value(operation).ok())
                    .collect(),
            })),
            Self::MessagesSnapshot { timestamp, messages } => {
                Ok(Event::MessagesSnapshot(MessagesSnapshotEvent {
                    base: BaseEvent {
                        timestamp: Some(timestamp),
                        raw_event: None,
                    },
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
                base: BaseEvent {
                    timestamp: Some(timestamp),
                    raw_event: None,
                },
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
                                    call_type: "function".to_string(), // TODO: What are the supported values here?
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

#[async_trait]
impl AgUiService for ApplicationService {
    type Error = ServiceError;

    async fn ag_ui_run(
        &self,
        input: RunAgentInput,
        tenant_id: &str,
    ) -> Result<BoxStream<'static, Result<Event, Self::Error>>, Self::Error> {
        let (stream, _) = self
            .run(
                RunParams::new(tenant_id, input.thread_id)
                    .with_run_id(input.run_id)
                    .maybe_with_model_override(
                        input
                            .forwarded_props
                            .as_object()
                            .and_then(|props| props.get("model_override"))
                            .and_then(|v| from_value(v.clone()).ok()),
                    )
                    .maybe_with_capability_override(
                        input
                            .forwarded_props
                            .as_object()
                            .and_then(|props| props.get("capability_override"))
                            .and_then(|v| from_value(v.clone()).ok()),
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
                    .with_tools(
                        input
                            .tools
                            .into_iter()
                            .map(|tool| ToolDefinition {
                                name: tool.name,
                                description: tool.description,
                                parameters: tool.parameters,
                            }),
                    )
                    .with_messages(
                        input
                            .messages
                            .into_iter()
                            .filter_map(|message| match message {
                                Message::System { id, content, name } => {
                                    Some(CreateMessageParams::System(CreateSystemMessageParams {
                                        id: id.into(),
                                        content,
                                        name,
                                    }))
                                }
                                Message::User { id, content, name } => {
                                    Some(CreateMessageParams::User(CreateUserMessageParams {
                                        id: id.into(),
                                        name,
                                        content: match content {
                                            UserMessageContent::Text(text) => {
                                                vec![DomainUserContent::Text(text)]
                                            }
                                            UserMessageContent::Parts(parts) => parts
                                                .into_iter()
                                                .filter_map(|block| {
                                                    DomainUserContent::from_ag_ui_type(block).ok()
                                                })
                                                .collect(),
                                        },
                                    }))
                                }
                                Message::Tool { id, content, tool_call_id, error } => {
                                    Some(CreateMessageParams::Tool(CreateToolMessageParams {
                                        id: id.into(),
                                        content: Some(content),
                                        tool_call_id: tool_call_id.into(),
                                        parent_message_id: None,
                                        error,
                                        name: None,
                                    }))
                                }
                                _ => None,
                            }),
                    ),
            )
            .await?;

        Ok(Box::pin(stream.map(|event| event.to_ag_ui_type())))
    }
}
