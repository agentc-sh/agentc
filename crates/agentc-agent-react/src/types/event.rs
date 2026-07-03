// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use json_patch::PatchOperation;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;

use agentc_agent::types::event::AgentEvent;
use agentc_domain::types::run::RunStatus;

use crate::{
    graph::state::{ReActState, ReActStateUpdate},
    types::message::Message,
};

/// The subtype of a [`Event::ReasoningSignature`] event, indicating
/// which kind of entity the encrypted reasoning is attached to.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReasoningSignatureSubtype {
    /// The encrypted reasoning is attached to a reasoning message.
    Message,
    /// The encrypted reasoning is attached to a tool call.
    ToolCall,
}

impl Display for ReasoningSignatureSubtype {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            ReasoningSignatureSubtype::Message => write!(f, "message"),
            ReasoningSignatureSubtype::ToolCall => write!(f, "tool_call"),
        }
    }
}

impl From<ReasoningSignatureSubtype> for String {
    fn from(subtype: ReasoningSignatureSubtype) -> Self {
        subtype.to_string()
    }
}

impl FromStr for ReasoningSignatureSubtype {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "message" => Ok(ReasoningSignatureSubtype::Message),
            "tool_call" => Ok(ReasoningSignatureSubtype::ToolCall),
            _ => Err(()),
        }
    }
}

/// An enum representing different types of events that can occur within the agent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Event {
    /// Event indicating that a run has started.
    RunStarted {
        timestamp: f64,
        session_id: Uuid,
        run_id: Uuid,
    },
    /// Event indicating that a run has finished.
    RunFinished {
        timestamp: f64,
        session_id: Uuid,
        run_id: Uuid,
        status: RunStatus,
        #[serde(skip_serializing_if = "Option::is_none")]
        interrupt_payload: Option<Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        result: Option<ReActState>,
    },
    /// Event indicating that a run has encountered an error.
    RunError {
        timestamp: f64,
        session_id: Uuid,
        run_id: Uuid,
        error: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        code: Option<String>,
    },
    /// Event indicating the start of a text message.
    TextMessageStart { timestamp: f64, message_id: Uuid },
    /// Event indicating the end of a text message.
    TextMessageEnd { timestamp: f64, message_id: Uuid },
    /// Event indicating a delta of content within a text message.
    TextMessageContent {
        timestamp: f64,
        message_id: Uuid,
        delta: String,
    },
    /// Event indicating the start of a tool call.
    ToolCallStart {
        timestamp: f64,
        tool_call_id: String,
        tool_name: String,
    },
    /// Event indicating the end of a tool call.
    ToolCallEnd {
        timestamp: f64,
        tool_call_id: String,
    },
    /// Event indicating a delta for arguments within a tool call.
    ToolCallArgs {
        timestamp: f64,
        tool_call_id: String,
        delta: String,
    },
    /// Event indicating an error within a tool call.
    ToolCallError {
        timestamp: f64,
        tool_call_id: String,
        message_id: Uuid,
        error: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        code: Option<String>,
    },
    /// Event indicating the result of a tool call.
    ToolCallResult {
        timestamp: f64,
        tool_call_id: String,
        message_id: Uuid,
        content: Value,
    },
    /// Event indicating a delta update to an tool call's activity
    ActivityDelta {
        timestamp: f64,
        tool_call_id: String,
        activity_type: String,
        patch: Vec<PatchOperation>,
    },
    /// Event marking the start of a reasoning phase.
    ReasoningStart { timestamp: f64, message_id: Uuid },
    /// Event marking the end of a reasoning phase.
    ReasoningEnd { timestamp: f64, message_id: Uuid },
    /// Event marking the start of a streaming reasoning message.
    ReasoningMessageStart { timestamp: f64, message_id: Uuid },
    /// Event delivering a chunk of visible reasoning content.
    ReasoningMessageContent {
        timestamp: f64,
        message_id: Uuid,
        delta: String,
    },
    /// Event marking the end of a streaming reasoning message.
    ReasoningMessageEnd { timestamp: f64, message_id: Uuid },
    /// Event attaching an encrypted chain-of-thought blob to a message or
    /// tool call for state continuity across conversation turns.
    ReasoningSignature {
        timestamp: f64,
        message_id: Uuid,
        subtype: ReasoningSignatureSubtype,
        entity_id: String,
        value: String,
    },
    /// Event indicating a full snapshot of the agent's state.
    StateSnapshot { timestamp: f64, state: ReActState },
    /// Event indicating a delta update to the agent's state.
    StateDelta {
        timestamp: f64,
        delta: ReActStateUpdate,
    },
    /// Event indicating a snapshot of all messages.
    MessagesSnapshot {
        timestamp: f64,
        messages: Vec<Message>,
    },
}

impl Event {
    /// Returns the kind of the event as a string.
    pub fn kind(&self) -> &str {
        match self {
            Event::RunStarted { .. } => "run_started",
            Event::RunFinished { .. } => "run_finished",
            Event::RunError { .. } => "run_error",
            Event::TextMessageStart { .. } => "text_message_start",
            Event::TextMessageEnd { .. } => "text_message_end",
            Event::TextMessageContent { .. } => "text_message_content",
            Event::ToolCallStart { .. } => "tool_call_start",
            Event::ToolCallEnd { .. } => "tool_call_end",
            Event::ToolCallArgs { .. } => "tool_call_args",
            Event::ToolCallError { .. } => "tool_call_error",
            Event::ToolCallResult { .. } => "tool_call_result",
            Event::ActivityDelta { .. } => "activity_delta",
            Event::ReasoningStart { .. } => "reasoning_start",
            Event::ReasoningEnd { .. } => "reasoning_end",
            Event::ReasoningMessageStart { .. } => "reasoning_message_start",
            Event::ReasoningMessageContent { .. } => "reasoning_message_content",
            Event::ReasoningMessageEnd { .. } => "reasoning_message_end",
            Event::ReasoningSignature { .. } => "reasoning_signature",
            Event::StateSnapshot { .. } => "state_snapshot",
            Event::StateDelta { .. } => "state_delta",
            Event::MessagesSnapshot { .. } => "messages_snapshot",
        }
    }

    /// Creates a new RunStarted event.
    ///
    /// # Arguments
    /// * `session_id` - A string slice representing the session ID.
    /// * `run_id` - A string slice representing the run ID.
    ///
    /// # Returns
    /// An instance of the Event enum representing a [`Event::RunStarted`](crate::types::event::Event::RunStarted) event.
    pub fn run_started(session_id: impl Into<Uuid>, run_id: impl Into<Uuid>) -> Self {
        Event::RunStarted {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs_f64(),
            session_id: session_id.into(),
            run_id: run_id.into(),
        }
    }

    /// Creates a new RunFinished event.
    ///
    /// # Arguments
    /// * `session_id` - A string slice representing the session ID.
    /// * `run_id` - A string slice representing the run ID.
    /// * `status` - A RunStatus value representing the status of the run.
    /// * `interrupt_payload` - An optional value representing the interrupt payload if the run was interrupted.
    /// * `result` - An optional value representing the result of the run.
    ///
    /// # Returns
    /// An instance of the Event enum representing a [`Event::RunFinished`](crate::types::event::Event::RunFinished) event.
    pub fn run_finished(
        session_id: impl Into<Uuid>,
        run_id: impl Into<Uuid>,
        status: RunStatus,
        interrupt_payload: Option<Value>,
        result: Option<ReActState>,
    ) -> Self {
        Event::RunFinished {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs_f64(),
            session_id: session_id.into(),
            run_id: run_id.into(),
            status,
            interrupt_payload,
            result,
        }
    }

    /// Creates a new RunError event.
    ///
    /// # Arguments
    /// * `session_id` - A string slice representing the session ID.
    /// * `run_id` - A string slice representing the run ID.
    /// * `error` - A string slice representing the error message.
    /// * `code` - An optional string slice representing the error code.
    ///
    /// # Returns
    /// An instance of the Event enum representing a [`Event::RunError`](crate::types::event::Event::RunError) event.
    pub fn run_error(
        session_id: impl Into<Uuid>,
        run_id: impl Into<Uuid>,
        error: impl Into<String>,
        code: Option<String>,
    ) -> Self {
        Event::RunError {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs_f64(),
            session_id: session_id.into(),
            run_id: run_id.into(),
            error: error.into(),
            code,
        }
    }

    /// Creates a new TextMessageStart event.
    ///
    /// # Arguments
    /// * `message_id` - A string slice representing the message ID.
    ///
    /// # Returns
    /// An instance of the Event enum representing a [`Event::TextMessageStart`](crate::types::event::Event::TextMessageStart) event.
    pub fn text_message_start(message_id: impl Into<Uuid>) -> Self {
        Event::TextMessageStart {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs_f64(),
            message_id: message_id.into(),
        }
    }

    /// Creates a new TextMessageEnd event.
    ///
    /// # Arguments
    /// * `message_id` - A string slice representing the message ID.
    ///
    /// # Returns
    /// An instance of the Event enum representing a [`Event::TextMessageEnd`](crate::types::event::Event::TextMessageEnd) event.
    pub fn text_message_end(message_id: impl Into<Uuid>) -> Self {
        Event::TextMessageEnd {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs_f64(),
            message_id: message_id.into(),
        }
    }

    /// Creates a new TextMessageContent event.
    ///
    /// # Arguments
    /// * `message_id` - A string slice representing the message ID.
    /// * `delta` - A string slice representing the content delta.
    ///
    /// # Returns
    /// An instance of the Event enum representing a [`Event::TextMessageContent`](crate::types::event::Event::TextMessageContent) event.
    pub fn text_message_content(message_id: impl Into<Uuid>, delta: impl Into<String>) -> Self {
        Event::TextMessageContent {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs_f64(),
            message_id: message_id.into(),
            delta: delta.into(),
        }
    }

    /// Creates a new ToolCallStart event.
    ///
    /// # Arguments
    /// * `tool_call_id` - A string slice representing the tool call ID.
    /// * `tool_name` - A string slice representing the tool name.
    ///
    /// # Returns
    /// An instance of the Event enum representing a [`Event::ToolCallStart`](crate::types::event::Event::ToolCallStart) event.
    pub fn tool_call_start(tool_call_id: impl Into<String>, tool_name: impl Into<String>) -> Self {
        Event::ToolCallStart {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs_f64(),
            tool_call_id: tool_call_id.into(),
            tool_name: tool_name.into(),
        }
    }

    /// Creates a new ToolCallEnd event.
    ///
    /// # Arguments
    /// * `tool_call_id` - A string slice representing the tool call ID.
    ///
    /// # Returns
    /// An instance of the Event enum representing a [`Event::ToolCallEnd`](crate::types::event::Event::ToolCallEnd) event.
    pub fn tool_call_end(tool_call_id: impl Into<String>) -> Self {
        Event::ToolCallEnd {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs_f64(),
            tool_call_id: tool_call_id.into(),
        }
    }

    /// Creates a new ToolCallArgs event.
    ///
    /// # Arguments
    /// * `tool_call_id` - A string slice representing the tool call ID.
    /// * `delta` - A string slice representing the arguments delta.
    ///
    /// # Returns
    /// An instance of the Event enum representing a [`Event::ToolCallArgs`](crate::types::event::Event::ToolCallArgs) event.
    pub fn tool_call_args(tool_call_id: impl Into<String>, delta: impl Into<String>) -> Self {
        Event::ToolCallArgs {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs_f64(),
            tool_call_id: tool_call_id.into(),
            delta: delta.into(),
        }
    }

    /// Creates a new ToolCallError event.
    ///
    /// # Arguments
    /// * `tool_call_id` - A string slice representing the tool call ID.
    /// * `message_id` - A string slice representing the message ID.
    /// * `error` - A string slice representing the error message.
    /// * `code` - An optional string slice representing the error code.
    ///
    /// # Returns
    /// An instance of the Event enum representing a [`Event::ToolCallError`](crate::types::event::Event::ToolCallError) event.
    pub fn tool_call_error(
        tool_call_id: impl Into<String>,
        message_id: impl Into<Uuid>,
        error: impl Into<String>,
        code: Option<impl Into<String>>,
    ) -> Self {
        Event::ToolCallError {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs_f64(),
            tool_call_id: tool_call_id.into(),
            message_id: message_id.into(),
            error: error.into(),
            code: code.map(|c| c.into()),
        }
    }

    /// Creates a new ToolCallResult event.
    ///
    /// # Arguments
    /// * `tool_call_id` - A string slice representing the tool call ID.
    /// * `message_id` - A string slice representing the message ID.
    /// * `content` - A value representing the content.
    ///
    /// # Returns
    /// An instance of the Event enum representing a [`Event::ToolCallResult`](crate::types::event::Event::ToolCallResult) event.
    pub fn tool_call_result(
        tool_call_id: impl Into<String>,
        message_id: impl Into<Uuid>,
        content: impl Into<Value>,
    ) -> Self {
        Event::ToolCallResult {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs_f64(),
            tool_call_id: tool_call_id.into(),
            message_id: message_id.into(),
            content: content.into(),
        }
    }

    /// Creates a new ActivityDelta event.
    ///
    /// # Arguments
    /// * `tool_call_id` - A string slice representing the tool call ID.
    /// * `activity_type` - A string slice representing the type of activity for the frontend.
    /// * `patch` - A vector of PatchOperation representing the JSON Patch operations to apply to the current activity.
    ///
    /// # Returns
    /// An instance of the Event enum representing a [`Event::ActivityDelta`](crate::types::event::Event::ActivityDelta) event.
    pub fn activity_delta(
        tool_call_id: impl Into<String>,
        activity_type: impl Into<String>,
        patch: Vec<PatchOperation>,
    ) -> Self {
        Event::ActivityDelta {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs_f64(),
            tool_call_id: tool_call_id.into(),
            activity_type: activity_type.into(),
            patch,
        }
    }

    /// Creates a new ReasoningStart event.
    ///
    /// # Arguments
    /// * `message_id` - A string slice representing the message ID associated with the reasoning
    ///
    /// # Returns
    /// An instance of the Event enum representing a [`Event::ReasoningStart`](crate::types::event::Event::ReasoningStart) event.
    pub fn reasoning_start(message_id: impl Into<Uuid>) -> Self {
        Event::ReasoningStart {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs_f64(),
            message_id: message_id.into(),
        }
    }

    /// Creates a new ReasoningEnd event.
    ///
    /// # Arguments
    /// * `message_id` - A string slice representing the message ID associated with the reasoning
    ///
    /// # Returns
    /// An instance of the Event enum representing a [`Event::ReasoningEnd`](crate::types::event::Event::ReasoningEnd) event.
    pub fn reasoning_end(message_id: impl Into<Uuid>) -> Self {
        Event::ReasoningEnd {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs_f64(),
            message_id: message_id.into(),
        }
    }

    /// Creates a new ReasoningMessageStart event.
    ///
    /// # Arguments
    /// * `message_id` - A string slice representing the message ID associated with the reasoning message
    ///
    /// # Returns
    /// An instance of the Event enum representing a [`Event::ReasoningMessageStart`](crate::types::event::Event::ReasoningMessageStart) event.
    pub fn reasoning_message_start(message_id: impl Into<Uuid>) -> Self {
        Event::ReasoningMessageStart {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs_f64(),
            message_id: message_id.into(),
        }
    }

    /// Creates a new ReasoningMessageContent event.
    ///
    /// # Arguments
    /// * `message_id` - A string slice representing the message ID associated with the reasoning message
    /// * `delta` - A string slice representing the content delta of the reasoning message
    ///
    /// # Returns
    /// An instance of the Event enum representing a [`Event::ReasoningMessageContent`](crate::types::event::Event::ReasoningMessageContent) event.
    pub fn reasoning_message_content(
        message_id: impl Into<Uuid>,
        delta: impl Into<String>,
    ) -> Self {
        Event::ReasoningMessageContent {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs_f64(),
            message_id: message_id.into(),
            delta: delta.into(),
        }
    }

    /// Creates a new ReasoningMessageEnd event.
    ///
    /// # Arguments
    /// * `message_id` - A string slice representing the message ID associated with the reasoning message
    ///
    /// # Returns
    /// An instance of the Event enum representing a [`Event::ReasoningMessageEnd`](crate::types::event::Event::ReasoningMessageEnd) event.
    pub fn reasoning_message_end(message_id: impl Into<Uuid>) -> Self {
        Event::ReasoningMessageEnd {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs_f64(),
            message_id: message_id.into(),
        }
    }

    /// Creates a new ReasoningSignature event.
    ///
    /// # Arguments
    /// * `message_id` - A string slice representing the message ID or tool call ID that the reasoning signature is associated with
    /// * `subtype` - A ReasoningSignatureSubtype value indicating whether the signature is attached to a message or a tool call
    /// * `entity_id` - A string slice representing the ID of the specific message or tool call entity that the reasoning signature is attached to.
    /// * `value` - A string slice representing the encrypted reasoning blob value.
    ///
    /// # Returns
    /// An instance of the Event enum representing a [`Event::ReasoningSignature`](crate::types::event::Event::ReasoningSignature) event.
    pub fn reasoning_signature(
        message_id: impl Into<Uuid>,
        subtype: ReasoningSignatureSubtype,
        entity_id: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        Event::ReasoningSignature {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs_f64(),
            message_id: message_id.into(),
            subtype,
            entity_id: entity_id.into(),
            value: value.into(),
        }
    }

    /// Creates a new StateSnapshot event.
    ///
    /// # Arguments
    /// * `state` - A value representing the state.
    ///
    /// # Returns
    /// An instance of the Event enum representing a [`Event::StateSnapshot`](crate::types::event::Event::StateSnapshot) event.
    pub fn state_snapshot(state: ReActState) -> Self {
        Event::StateSnapshot {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs_f64(),
            state,
        }
    }

    /// Creates a new StateDelta event.
    ///
    /// # Arguments
    /// * `delta` - The state update to patch the current state with.
    ///
    /// # Returns
    /// An instance of the Event enum representing a [`Event::StateDelta`](crate::types::event::Event::StateDelta) event.
    pub fn state_delta(delta: ReActStateUpdate) -> Self {
        Event::StateDelta {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs_f64(),
            delta,
        }
    }

    /// Creates a new MessagesSnapshot event.
    ///
    /// # Arguments
    /// * `messages` - A vector of Message instances representing the messages snapshot.
    ///
    /// # Returns
    /// An instance of the Event enum representing a [`Event::MessagesSnapshot`](crate::types::event::Event::MessagesSnapshot) event.
    pub fn messages_snapshot(messages: Vec<Message>) -> Self {
        Event::MessagesSnapshot {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs_f64(),
            messages,
        }
    }
}

impl From<AgentEvent<ReActState>> for Event {
    fn from(event: AgentEvent<ReActState>) -> Self {
        match event {
            AgentEvent::RunStarted { session_id, run_id, .. } => {
                Event::run_started(session_id, run_id)
            }
            AgentEvent::RunFinished {
                session_id,
                run_id,
                status,
                interrupt_payload,
                result,
                ..
            } => Event::run_finished(
                session_id,
                run_id,
                if status.is_interrupted() {
                    RunStatus::Interrupted
                } else {
                    RunStatus::Completed
                },
                interrupt_payload,
                result,
            ),
            AgentEvent::RunError { session_id, run_id, error, code, .. } => {
                Event::run_error(session_id, run_id, error, code)
            }
        }
    }
}
