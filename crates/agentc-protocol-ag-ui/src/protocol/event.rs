// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

use crate::protocol::{
    ids::{MessageId, RunId, ThreadId, ToolCallId},
    message::{Message, Role},
    state::AgentState,
};

/// Event types for AG-UI protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EventType {
    /// Event indicating the start of a text message
    TextMessageStart,
    /// Event containing a piece of text message content
    TextMessageContent,
    /// Event indicating the end of a text message
    TextMessageEnd,
    /// Event containing a chunk of text message content
    TextMessageChunk,
    /// Event indicating the start of a tool call
    ToolCallStart,
    /// Event containing tool call arguments
    ToolCallArgs,
    /// Event indicating the end of a tool call
    ToolCallEnd,
    /// Event containing a chunk of tool call content
    ToolCallChunk,
    /// Event containing the result of a tool call
    ToolCallResult,
    /// Event marking the start of a reasoning phase
    ReasoningStart,
    /// Event indicating the start of a streaming reasoning message
    ReasoningMessageStart,
    /// Event containing a chunk of reasoning message content
    ReasoningMessageContent,
    /// Event indicating the end of a reasoning message
    ReasoningMessageEnd,
    /// Convenience event that auto-manages reasoning message lifecycle
    ReasoningMessageChunk,
    /// Event marking the end of a reasoning phase
    ReasoningEnd,
    /// Event attaching encrypted chain-of-thought to a message or tool call
    ReasoningEncryptedValue,
    /// Event containing a snapshot of the state
    StateSnapshot,
    /// Event containing a delta of the state
    StateDelta,
    /// Event containing a snapshot of the messages
    MessagesSnapshot,
    /// Event containing a raw event
    Raw,
    /// Event containing a custom event
    Custom,
    /// Event indicating that a run has started
    RunStarted,
    /// Event indicating that a run has finished
    RunFinished,
    /// Event indicating that a run has encountered an error
    RunError,
    /// Event indicating that a step has started
    StepStarted,
    /// Event indicating that a step has finished
    StepFinished,
}

impl EventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            EventType::TextMessageStart => "TEXT_MESSAGE_START",
            EventType::TextMessageContent => "TEXT_MESSAGE_CONTENT",
            EventType::TextMessageEnd => "TEXT_MESSAGE_END",
            EventType::TextMessageChunk => "TEXT_MESSAGE_CHUNK",
            EventType::ToolCallStart => "TOOL_CALL_START",
            EventType::ToolCallArgs => "TOOL_CALL_ARGS",
            EventType::ToolCallEnd => "TOOL_CALL_END",
            EventType::ToolCallChunk => "TOOL_CALL_CHUNK",
            EventType::ToolCallResult => "TOOL_CALL_RESULT",
            EventType::ReasoningStart => "REASONING_START",
            EventType::ReasoningMessageStart => "REASONING_MESSAGE_START",
            EventType::ReasoningMessageContent => "REASONING_MESSAGE_CONTENT",
            EventType::ReasoningMessageEnd => "REASONING_MESSAGE_END",
            EventType::ReasoningMessageChunk => "REASONING_MESSAGE_CHUNK",
            EventType::ReasoningEnd => "REASONING_END",
            EventType::ReasoningEncryptedValue => "REASONING_ENCRYPTED_VALUE",
            EventType::StateSnapshot => "STATE_SNAPSHOT",
            EventType::StateDelta => "STATE_DELTA",
            EventType::MessagesSnapshot => "MESSAGES_SNAPSHOT",
            EventType::Raw => "RAW",
            EventType::Custom => "CUSTOM",
            EventType::RunStarted => "RUN_STARTED",
            EventType::RunFinished => "RUN_FINISHED",
            EventType::RunError => "RUN_ERROR",
            EventType::StepStarted => "STEP_STARTED",
            EventType::StepFinished => "STEP_FINISHED",
        }
    }
}

/// Base event for all events in the Agent User Interaction Protocol.
/// Contains common fields that are present in all event types.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct BaseEvent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<f64>,
    #[serde(rename = "rawEvent", skip_serializing_if = "Option::is_none")]
    pub raw_event: Option<Value>,
}

/// Event indicating the start of a text message.
/// This event is sent when the agent begins generating a text message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct TextMessageStartEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    #[serde(rename = "messageId")]
    pub message_id: MessageId,
    pub role: Role, // "assistant"
}

/// Event containing a piece of text message content.
/// This event is sent for each chunk of content as the agent generates a message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct TextMessageContentEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    #[serde(rename = "messageId")]
    pub message_id: MessageId,
    pub delta: String,
}

/// Event indicating the end of a text message.
/// This event is sent when the agent completes a text message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct TextMessageEndEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    #[serde(rename = "messageId")]
    pub message_id: MessageId,
}

/// Event containing a chunk of text message content.
/// This event combines start, content, and potentially end information in a single event,
/// with optional fields that may or may not be present.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct TextMessageChunkEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    #[serde(rename = "messageId", skip_serializing_if = "Option::is_none")]
    pub message_id: Option<MessageId>,
    pub role: Role,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta: Option<String>,
}

/// Event marking the start of a reasoning phase.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ReasoningStartEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    #[serde(rename = "messageId")]
    pub message_id: MessageId,
}

/// Event indicating the start of a streaming reasoning message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ReasoningMessageStartEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    #[serde(rename = "messageId")]
    pub message_id: MessageId,
    pub role: Role, // "reasoning"
}

/// Event containing a chunk of reasoning message content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ReasoningMessageContentEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    #[serde(rename = "messageId")]
    pub message_id: MessageId,
    pub delta: String,
}

/// Event indicating the end of a reasoning message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ReasoningMessageEndEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    #[serde(rename = "messageId")]
    pub message_id: MessageId,
}

/// Convenience event that auto-manages reasoning message lifecycle.
/// An empty delta or the next non-reasoning event implicitly closes the message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ReasoningMessageChunkEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    #[serde(rename = "messageId")]
    pub message_id: MessageId,
    pub delta: String,
}

/// Event marking the end of a reasoning phase.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ReasoningEndEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    #[serde(rename = "messageId")]
    pub message_id: MessageId,
}

/// The entity type that an encrypted reasoning value is attached to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ReasoningEncryptedValueSubtype {
    /// Encrypted reasoning attached to a reasoning or assistant message.
    Message,
    /// Encrypted reasoning attached to a tool call.
    ToolCall,
}

/// Event attaching encrypted chain-of-thought to a message or tool call.
/// The client stores and forwards the encrypted value opaquely on subsequent turns.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ReasoningEncryptedValueEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    pub subtype: ReasoningEncryptedValueSubtype,
    #[serde(rename = "entityId")]
    pub entity_id: String,
    #[serde(rename = "encryptedValue")]
    pub encrypted_value: String,
}

/// Event indicating the start of a tool call.
/// This event is sent when the agent begins to call a tool with specific parameters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ToolCallStartEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    #[serde(rename = "toolCallId")]
    pub tool_call_id: ToolCallId,
    #[serde(rename = "toolCallName")]
    pub tool_call_name: String,
    #[serde(rename = "parentMessageId", skip_serializing_if = "Option::is_none")]
    pub parent_message_id: Option<MessageId>,
}

/// Event containing tool call arguments.
/// This event contains chunks of the arguments being passed to a tool.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ToolCallArgsEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    #[serde(rename = "toolCallId")]
    pub tool_call_id: ToolCallId,
    pub delta: String,
}

/// Event indicating the end of a tool call.
/// This event is sent when the agent completes sending arguments to a tool.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ToolCallEndEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    #[serde(rename = "toolCallId")]
    pub tool_call_id: ToolCallId,
}

/// Event containing the result of a tool call.
/// This event is sent when a tool has completed execution and returns its result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ToolCallResultEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    #[serde(rename = "messageId")]
    pub message_id: MessageId,
    #[serde(rename = "toolCallId")]
    pub tool_call_id: ToolCallId,
    pub content: String,
    #[serde(default = "Role::tool")]
    pub role: Role, // "tool"
}

/// Event containing a chunk of tool call content.
/// This event combines start, args, and potentially end information in a single event,
/// with optional fields that may or may not be present.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ToolCallChunkEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    #[serde(rename = "toolCallId", skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<ToolCallId>,
    #[serde(rename = "toolCallName", skip_serializing_if = "Option::is_none")]
    pub tool_call_name: Option<String>,
    #[serde(rename = "parentMessageId", skip_serializing_if = "Option::is_none")]
    pub parent_message_id: Option<MessageId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta: Option<String>,
}

/// Event containing a snapshot of the state.
/// This event provides a complete representation of the current agent state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(bound(deserialize = ""))]
pub struct StateSnapshotEvent<StateT: AgentState = Value> {
    #[serde(flatten)]
    pub base: BaseEvent,
    pub snapshot: StateT,
}

/// Event containing a delta of the state.
/// This event contains JSON Patch operations (RFC 6902) that describe changes to the agent state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct StateDeltaEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    pub delta: Vec<Value>,
}

/// Event containing a snapshot of the messages.
/// This event provides a complete list of all current conversation messages.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct MessagesSnapshotEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    pub messages: Vec<Message>,
}

/// Event containing a raw event.
/// This event type allows wrapping arbitrary events from external sources.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct RawEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    pub event: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// Event containing a custom event.
/// This event type allows for application-specific custom events with arbitrary data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct CustomEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    pub name: String,
    pub value: Value,
}

/// Event indicating that a run has started.
/// This event is sent when an agent run begins execution within a specific thread.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct RunStartedEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId")]
    pub run_id: RunId,
}

/// Event indicating that a run has finished.
/// This event is sent when an agent run completes successfully, potentially with a result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct RunFinishedEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId")]
    pub run_id: RunId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
}

/// Event indicating that a run has encountered an error.
/// This event is sent when an agent run fails with an error message and optional error code.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct RunErrorEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

/// Event indicating that a step has started.
/// This event is sent when a specific named step within a run begins execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct StepStartedEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    #[serde(rename = "stepName")]
    pub step_name: String,
}

/// Event indicating that a step has finished.
/// This event is sent when a specific named step within a run completes execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct StepFinishedEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    #[serde(rename = "stepName")]
    pub step_name: String,
}

/// Union of all possible events in the Agent User Interaction Protocol.
/// This enum represents the full set of events that can be exchanged
/// between the agent and the client.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(
    tag = "type",
    rename_all = "SCREAMING_SNAKE_CASE",
    bound(deserialize = "")
)]
pub enum Event<StateT: AgentState = Value> {
    /// Signals the start of a text message from an agent.
    /// Contains the message ID and role information.
    TextMessageStart(TextMessageStartEvent),

    /// Represents a chunk of content being added to an in-progress text message.
    /// Contains the message ID and the text delta to append.
    TextMessageContent(TextMessageContentEvent),

    /// Signals the completion of a text message.
    /// Contains the message ID of the completed message.
    TextMessageEnd(TextMessageEndEvent),

    /// Represents a complete or partial message chunk in a single event.
    /// May contain optional message ID, role, and delta information.
    TextMessageChunk(TextMessageChunkEvent),

    /// Signals the start of a tool call by the agent.
    /// Contains the tool call ID, name, and optional parent message ID.
    ToolCallStart(ToolCallStartEvent),

    /// Represents arguments being added to an in-progress tool call.
    /// Contains the tool call ID and argument data delta.
    ToolCallArgs(ToolCallArgsEvent),

    /// Signals the completion of a tool call.
    /// Contains the tool call ID of the completed call.
    ToolCallEnd(ToolCallEndEvent),

    /// Represents a complete or partial tool call in a single event.
    /// May contain optional tool call ID, name, parent message ID, and delta.
    ToolCallChunk(ToolCallChunkEvent),

    /// Represents the result of a completed tool call.
    /// Contains the message ID, tool call ID, content, and optional role.
    ToolCallResult(ToolCallResultEvent),

    /// Marks the start of a reasoning phase.
    ReasoningStart(ReasoningStartEvent),

    /// Begins a streaming reasoning message within a reasoning phase.
    ReasoningMessageStart(ReasoningMessageStartEvent),

    /// Delivers a chunk of reasoning message content.
    ReasoningMessageContent(ReasoningMessageContentEvent),

    /// Signals the completion of a reasoning message.
    ReasoningMessageEnd(ReasoningMessageEndEvent),

    /// Convenience event that auto-manages reasoning message lifecycle.
    ReasoningMessageChunk(ReasoningMessageChunkEvent),

    /// Marks the end of a reasoning phase.
    ReasoningEnd(ReasoningEndEvent),

    /// Attaches encrypted chain-of-thought to a message or tool call.
    ReasoningEncryptedValue(ReasoningEncryptedValueEvent),

    /// Provides a complete snapshot of the current state.
    /// Contains the full state as a JSON value.
    StateSnapshot(StateSnapshotEvent<StateT>),

    /// Provides incremental changes to the state.
    /// Contains a vector of delta operations to apply to the state.
    StateDelta(StateDeltaEvent),

    /// Provides a complete snapshot of all messages.
    /// Contains a vector of all current messages.
    MessagesSnapshot(MessagesSnapshotEvent),

    /// Wraps a raw event from an external source.
    /// Contains the original event as a JSON value and an optional source identifier.
    Raw(RawEvent),

    /// Represents a custom event type not covered by the standard events.
    /// Contains a name identifying the custom event type and an associated value.
    Custom(CustomEvent),

    /// Signals the start of an agent run.
    /// Contains thread ID and run ID to identify the run.
    RunStarted(RunStartedEvent),

    /// Signals the completion of an agent run.
    /// Contains thread ID, run ID, and optional result data.
    RunFinished(RunFinishedEvent),

    /// Signals an error that occurred during an agent run.
    /// Contains error message and optional error code.
    RunError(RunErrorEvent),

    /// Signals the start of a step within an agent run.
    /// Contains the name of the step being started.
    StepStarted(StepStartedEvent),

    /// Signals the completion of a step within an agent run.
    /// Contains the name of the completed step.
    StepFinished(StepFinishedEvent),
}

impl Event {
    /// Get the event type
    pub fn event_type(&self) -> EventType {
        match self {
            Event::TextMessageStart(_) => EventType::TextMessageStart,
            Event::TextMessageContent(_) => EventType::TextMessageContent,
            Event::TextMessageEnd(_) => EventType::TextMessageEnd,
            Event::TextMessageChunk(_) => EventType::TextMessageChunk,
            Event::ToolCallStart(_) => EventType::ToolCallStart,
            Event::ToolCallArgs(_) => EventType::ToolCallArgs,
            Event::ToolCallEnd(_) => EventType::ToolCallEnd,
            Event::ToolCallChunk(_) => EventType::ToolCallChunk,
            Event::ToolCallResult(_) => EventType::ToolCallResult,
            Event::ReasoningStart(_) => EventType::ReasoningStart,
            Event::ReasoningMessageStart(_) => EventType::ReasoningMessageStart,
            Event::ReasoningMessageContent(_) => EventType::ReasoningMessageContent,
            Event::ReasoningMessageEnd(_) => EventType::ReasoningMessageEnd,
            Event::ReasoningMessageChunk(_) => EventType::ReasoningMessageChunk,
            Event::ReasoningEnd(_) => EventType::ReasoningEnd,
            Event::ReasoningEncryptedValue(_) => EventType::ReasoningEncryptedValue,
            Event::StateSnapshot(_) => EventType::StateSnapshot,
            Event::StateDelta(_) => EventType::StateDelta,
            Event::MessagesSnapshot(_) => EventType::MessagesSnapshot,
            Event::Raw(_) => EventType::Raw,
            Event::Custom(_) => EventType::Custom,
            Event::RunStarted(_) => EventType::RunStarted,
            Event::RunFinished(_) => EventType::RunFinished,
            Event::RunError(_) => EventType::RunError,
            Event::StepStarted(_) => EventType::StepStarted,
            Event::StepFinished(_) => EventType::StepFinished,
        }
    }

    /// Get the timestamp if available
    pub fn timestamp(&self) -> Option<f64> {
        match self {
            Event::TextMessageStart(e) => e.base.timestamp,
            Event::TextMessageContent(e) => e.base.timestamp,
            Event::TextMessageEnd(e) => e.base.timestamp,
            Event::TextMessageChunk(e) => e.base.timestamp,
            Event::ToolCallStart(e) => e.base.timestamp,
            Event::ToolCallArgs(e) => e.base.timestamp,
            Event::ToolCallEnd(e) => e.base.timestamp,
            Event::ToolCallChunk(e) => e.base.timestamp,
            Event::ToolCallResult(e) => e.base.timestamp,
            Event::ReasoningStart(e) => e.base.timestamp,
            Event::ReasoningMessageStart(e) => e.base.timestamp,
            Event::ReasoningMessageContent(e) => e.base.timestamp,
            Event::ReasoningMessageEnd(e) => e.base.timestamp,
            Event::ReasoningMessageChunk(e) => e.base.timestamp,
            Event::ReasoningEnd(e) => e.base.timestamp,
            Event::ReasoningEncryptedValue(e) => e.base.timestamp,
            Event::StateSnapshot(e) => e.base.timestamp,
            Event::StateDelta(e) => e.base.timestamp,
            Event::MessagesSnapshot(e) => e.base.timestamp,
            Event::Raw(e) => e.base.timestamp,
            Event::Custom(e) => e.base.timestamp,
            Event::RunStarted(e) => e.base.timestamp,
            Event::RunFinished(e) => e.base.timestamp,
            Event::RunError(e) => e.base.timestamp,
            Event::StepStarted(e) => e.base.timestamp,
            Event::StepFinished(e) => e.base.timestamp,
        }
    }
}

/// Validation error types for events in the Agent User Interaction Protocol.
/// These errors represent validation failures when creating or processing events.
#[derive(Debug, thiserror::Error)]
pub enum EventValidationError {
    #[error("Delta must not be an empty string")]
    EmptyDelta,
    #[error("Invalid event format: {0}")]
    InvalidFormat(String),
}

/// Validate text message content event
impl TextMessageContentEvent {
    pub fn validate(&self) -> Result<(), EventValidationError> {
        if self.delta.is_empty() {
            return Err(EventValidationError::EmptyDelta);
        }
        Ok(())
    }
}

/// Builder pattern for creating events
impl TextMessageStartEvent {
    pub fn new(message_id: impl Into<MessageId>) -> Self {
        Self {
            base: BaseEvent { timestamp: None, raw_event: None },
            message_id: message_id.into(),
            role: Role::Assistant,
        }
    }

    pub fn with_timestamp(mut self, timestamp: f64) -> Self {
        self.base.timestamp = Some(timestamp);
        self
    }

    pub fn with_raw_event(mut self, raw_event: Value) -> Self {
        self.base.raw_event = Some(raw_event);
        self
    }
}

impl TextMessageContentEvent {
    pub fn new(
        message_id: impl Into<MessageId>,
        delta: String,
    ) -> Result<Self, EventValidationError> {
        let event = Self {
            base: BaseEvent { timestamp: None, raw_event: None },
            message_id: message_id.into(),
            delta,
        };
        event.validate()?;
        Ok(event)
    }

    pub fn with_timestamp(mut self, timestamp: f64) -> Self {
        self.base.timestamp = Some(timestamp);
        self
    }
}
