// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use chrono::{DateTime, Utc};
use futures::stream::Stream;
use json_patch::PatchOperation;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    pin::Pin,
    task::{Context, Poll},
};
use uuid::Uuid;

use agentc_agent::{
    stream::EventStream,
    types::{capability::CapabilityOverride, tools::ToolDefinition},
};
use agentc_domain::{
    repository::run::params::FindRunParams as RepoFindRunParams,
    types::{Run, RunStatus},
};

use crate::{
    graph::state::{ReActState, ReActStateInput, ReActStateUpdate},
    service::types::message::{CreateMessageParams, MessageResponse},
    types::model::ModelOverride,
    types::{
        context_var::ContextVar,
        event::{Event, ReasoningSignatureSubtype},
    },
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateResponse {
    pub run_id: Uuid,
    pub session_id: Uuid,
    pub model_override: Option<ModelOverride>,
    pub capability_override: Option<CapabilityOverride>,
    pub messages: Vec<MessageResponse>,
    pub context_vars: Vec<ContextVar>,
    pub context: Value,
    pub tools: Vec<ToolDefinition>,
}

impl StateResponse {
    pub fn from_entity(entity: &ReActState) -> Self {
        Self {
            run_id: entity.run_id,
            session_id: entity.session_id,
            model_override: entity.model_override.clone(),
            capability_override: entity.capability_override.clone(),
            messages: entity
                .messages
                .iter()
                .map(MessageResponse::from_entity)
                .collect(),
            context_vars: entity.context_vars.clone(),
            context: entity.context.clone(),
            tools: entity.tools.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateUpdateResponse {
    pub messages: Vec<MessageResponse>,
    pub context: Vec<PatchOperation>,
}

impl StateUpdateResponse {
    pub fn from_entity(entity: &ReActStateUpdate) -> Self {
        Self {
            messages: entity
                .messages
                .iter()
                .map(MessageResponse::from_entity)
                .collect(),
            context: entity.context.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RunEvent {
    RunStarted {
        timestamp: f64,
        session_id: Uuid,
        run_id: Uuid,
    },
    RunFinished {
        timestamp: f64,
        session_id: Uuid,
        run_id: Uuid,
        status: RunStatus,
        interrupt_payload: Option<Value>,
        result: Option<StateResponse>,
    },
    RunError {
        timestamp: f64,
        session_id: Uuid,
        run_id: Uuid,
        error: String,
        code: Option<String>,
    },
    TextMessageStart {
        timestamp: f64,
        message_id: Uuid,
    },
    TextMessageEnd {
        timestamp: f64,
        message_id: Uuid,
    },
    TextMessageContent {
        timestamp: f64,
        message_id: Uuid,
        delta: String,
    },
    ToolCallStart {
        timestamp: f64,
        tool_call_id: String,
        tool_name: String,
    },
    ToolCallEnd {
        timestamp: f64,
        tool_call_id: String,
    },
    ToolCallArgs {
        timestamp: f64,
        tool_call_id: String,
        delta: String,
    },
    ToolCallError {
        timestamp: f64,
        tool_call_id: String,
        message_id: Uuid,
        error: String,
        code: Option<String>,
    },
    ToolCallResult {
        timestamp: f64,
        tool_call_id: String,
        message_id: Uuid,
        content: Value,
    },
    ActivityDelta {
        timestamp: f64,
        tool_call_id: String,
        activity_type: String,
        patch: Vec<PatchOperation>,
    },
    ReasoningStart {
        timestamp: f64,
        message_id: Uuid,
    },
    ReasoningEnd {
        timestamp: f64,
        message_id: Uuid,
    },
    ReasoningMessageStart {
        timestamp: f64,
        message_id: Uuid,
    },
    ReasoningMessageContent {
        timestamp: f64,
        message_id: Uuid,
        delta: String,
    },
    ReasoningMessageEnd {
        timestamp: f64,
        message_id: Uuid,
    },
    ReasoningSignature {
        timestamp: f64,
        message_id: Uuid,
        subtype: ReasoningSignatureSubtype,
        entity_id: String,
        value: String,
    },
    StateSnapshot {
        timestamp: f64,
        state: StateResponse,
    },
    StateDelta {
        timestamp: f64,
        delta: StateUpdateResponse,
    },
    MessagesSnapshot {
        timestamp: f64,
        messages: Vec<MessageResponse>,
    },
}

impl RunEvent {
    pub fn kind(&self) -> &'static str {
        match self {
            RunEvent::RunStarted { .. } => "run_started",
            RunEvent::RunFinished { .. } => "run_finished",
            RunEvent::RunError { .. } => "run_error",
            RunEvent::TextMessageStart { .. } => "text_message_start",
            RunEvent::TextMessageEnd { .. } => "text_message_end",
            RunEvent::TextMessageContent { .. } => "text_message_content",
            RunEvent::ToolCallStart { .. } => "tool_call_start",
            RunEvent::ToolCallEnd { .. } => "tool_call_end",
            RunEvent::ToolCallArgs { .. } => "tool_call_args",
            RunEvent::ToolCallError { .. } => "tool_call_error",
            RunEvent::ToolCallResult { .. } => "tool_call_result",
            RunEvent::ActivityDelta { .. } => "activity_delta",
            RunEvent::ReasoningStart { .. } => "reasoning_start",
            RunEvent::ReasoningEnd { .. } => "reasoning_end",
            RunEvent::ReasoningMessageStart { .. } => "reasoning_message_start",
            RunEvent::ReasoningMessageContent { .. } => "reasoning_message_content",
            RunEvent::ReasoningMessageEnd { .. } => "reasoning_message_end",
            RunEvent::ReasoningSignature { .. } => "reasoning_signature",
            RunEvent::StateSnapshot { .. } => "state_snapshot",
            RunEvent::StateDelta { .. } => "state_delta",
            RunEvent::MessagesSnapshot { .. } => "messages_snapshot",
        }
    }

    pub fn from_entity(entity: &Event) -> Self {
        match entity {
            Event::RunStarted { timestamp, session_id, run_id } => Self::RunStarted {
                timestamp: *timestamp,
                session_id: *session_id,
                run_id: *run_id,
            },
            Event::RunFinished {
                timestamp,
                session_id,
                run_id,
                status,
                interrupt_payload,
                result,
            } => Self::RunFinished {
                timestamp: *timestamp,
                session_id: *session_id,
                run_id: *run_id,
                status: *status,
                interrupt_payload: interrupt_payload.clone(),
                result: result
                    .as_ref()
                    .map(StateResponse::from_entity),
            },
            Event::RunError {
                timestamp,
                session_id,
                run_id,
                error,
                code,
            } => Self::RunError {
                timestamp: *timestamp,
                session_id: *session_id,
                run_id: *run_id,
                error: error.clone(),
                code: code.clone(),
            },
            Event::TextMessageStart { timestamp, message_id } => Self::TextMessageStart {
                timestamp: *timestamp,
                message_id: *message_id,
            },
            Event::TextMessageEnd { timestamp, message_id } => Self::TextMessageEnd {
                timestamp: *timestamp,
                message_id: *message_id,
            },
            Event::TextMessageContent { timestamp, message_id, delta } => {
                Self::TextMessageContent {
                    timestamp: *timestamp,
                    message_id: *message_id,
                    delta: delta.clone(),
                }
            }
            Event::ToolCallStart { timestamp, tool_call_id, tool_name } => Self::ToolCallStart {
                timestamp: *timestamp,
                tool_call_id: tool_call_id.clone(),
                tool_name: tool_name.clone(),
            },
            Event::ToolCallEnd { timestamp, tool_call_id } => Self::ToolCallEnd {
                timestamp: *timestamp,
                tool_call_id: tool_call_id.clone(),
            },
            Event::ToolCallArgs { timestamp, tool_call_id, delta } => Self::ToolCallArgs {
                timestamp: *timestamp,
                tool_call_id: tool_call_id.clone(),
                delta: delta.clone(),
            },
            Event::ToolCallError {
                timestamp,
                tool_call_id,
                message_id,
                error,
                code,
            } => Self::ToolCallError {
                timestamp: *timestamp,
                tool_call_id: tool_call_id.clone(),
                message_id: *message_id,
                error: error.clone(),
                code: code.clone(),
            },
            Event::ToolCallResult {
                timestamp,
                tool_call_id,
                message_id,
                content,
            } => Self::ToolCallResult {
                timestamp: *timestamp,
                tool_call_id: tool_call_id.clone(),
                message_id: *message_id,
                content: content.clone(),
            },
            Event::ActivityDelta {
                timestamp,
                tool_call_id,
                activity_type,
                patch,
            } => Self::ActivityDelta {
                timestamp: *timestamp,
                tool_call_id: tool_call_id.clone(),
                activity_type: activity_type.clone(),
                patch: patch.clone(),
            },
            Event::ReasoningStart { timestamp, message_id } => Self::ReasoningStart {
                timestamp: *timestamp,
                message_id: *message_id,
            },
            Event::ReasoningEnd { timestamp, message_id } => Self::ReasoningEnd {
                timestamp: *timestamp,
                message_id: *message_id,
            },
            Event::ReasoningMessageStart { timestamp, message_id } => Self::ReasoningMessageStart {
                timestamp: *timestamp,
                message_id: *message_id,
            },
            Event::ReasoningMessageContent { timestamp, message_id, delta } => {
                Self::ReasoningMessageContent {
                    timestamp: *timestamp,
                    message_id: *message_id,
                    delta: delta.clone(),
                }
            }
            Event::ReasoningMessageEnd { timestamp, message_id } => Self::ReasoningMessageEnd {
                timestamp: *timestamp,
                message_id: *message_id,
            },
            Event::ReasoningSignature {
                timestamp,
                message_id,
                subtype,
                entity_id,
                value,
            } => Self::ReasoningSignature {
                timestamp: *timestamp,
                message_id: *message_id,
                subtype: subtype.clone(),
                entity_id: entity_id.clone(),
                value: value.clone(),
            },
            Event::StateSnapshot { timestamp, state } => Self::StateSnapshot {
                timestamp: *timestamp,
                state: StateResponse::from_entity(state),
            },
            Event::StateDelta { timestamp, delta } => Self::StateDelta {
                timestamp: *timestamp,
                delta: StateUpdateResponse::from_entity(delta),
            },
            Event::MessagesSnapshot { timestamp, messages } => Self::MessagesSnapshot {
                timestamp: *timestamp,
                messages: messages
                    .iter()
                    .map(MessageResponse::from_entity)
                    .collect(),
            },
        }
    }
}

pub struct RunStream {
    inner: EventStream<Event>,
}

impl RunStream {
    pub fn new(inner: EventStream<Event>) -> Self {
        Self { inner }
    }
}

impl Stream for RunStream {
    type Item = RunEvent;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match Pin::new(&mut self.inner).poll_next(cx) {
            Poll::Ready(Some(event)) => Poll::Ready(Some(RunEvent::from_entity(&event))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunParams {
    pub tenant_id: String,
    pub session_id: Uuid,
    pub run_id: Uuid,
    pub checkpoint_id: Option<Uuid>,
    pub resume_payload: Option<Value>,
    pub model_override: Option<ModelOverride>,
    pub capability_override: Option<CapabilityOverride>,
    pub messages: Vec<CreateMessageParams>,
    pub context_vars: Vec<ContextVar>,
    pub tools: Vec<ToolDefinition>,
    pub context: Option<Value>,
}

impl RunParams {
    pub fn new(tenant_id: impl Into<String>, session_id: impl Into<Uuid>) -> Self {
        Self {
            tenant_id: tenant_id.into(),
            session_id: session_id.into(),
            run_id: Uuid::new_v4(),
            checkpoint_id: None,
            resume_payload: None,
            model_override: None,
            capability_override: None,
            messages: Vec::new(),
            context_vars: Vec::new(),
            tools: Vec::new(),
            context: None,
        }
    }

    pub fn with_run_id(mut self, run_id: impl Into<Uuid>) -> Self {
        self.run_id = run_id.into();
        self
    }

    pub fn maybe_with_run_id(mut self, run_id: Option<impl Into<Uuid>>) -> Self {
        if let Some(run_id) = run_id {
            self.run_id = run_id.into();
        }
        self
    }

    pub fn with_checkpoint_id(mut self, checkpoint_id: impl Into<Uuid>) -> Self {
        self.checkpoint_id = Some(checkpoint_id.into());
        self
    }

    pub fn maybe_with_checkpoint_id(mut self, checkpoint_id: Option<impl Into<Uuid>>) -> Self {
        self.checkpoint_id = checkpoint_id.map(Into::into);
        self
    }

    pub fn with_resume_payload(mut self, resume_payload: impl Into<Value>) -> Self {
        self.resume_payload = Some(resume_payload.into());
        self
    }

    pub fn maybe_with_resume_payload(mut self, resume_payload: Option<impl Into<Value>>) -> Self {
        self.resume_payload = resume_payload.map(Into::into);
        self
    }

    pub fn with_model_override(mut self, model_override: ModelOverride) -> Self {
        self.model_override = Some(model_override);
        self
    }

    pub fn maybe_with_model_override(mut self, model_override: Option<ModelOverride>) -> Self {
        self.model_override = model_override;
        self
    }

    pub fn with_capability_override(mut self, capability_override: CapabilityOverride) -> Self {
        self.capability_override = Some(capability_override);
        self
    }

    pub fn maybe_with_capability_override(
        mut self,
        capability_override: Option<CapabilityOverride>,
    ) -> Self {
        self.capability_override = capability_override;
        self
    }

    pub fn with_messages(
        mut self,
        messages: impl IntoIterator<Item = CreateMessageParams>,
    ) -> Self {
        self.messages = messages.into_iter().collect();
        self
    }

    pub fn maybe_with_messages(
        mut self,
        messages: Option<impl IntoIterator<Item = CreateMessageParams>>,
    ) -> Self {
        if let Some(messages) = messages {
            self.messages = messages.into_iter().collect();
        }
        self
    }

    pub fn with_context_vars(mut self, context_vars: impl IntoIterator<Item = ContextVar>) -> Self {
        self.context_vars = context_vars.into_iter().collect();
        self
    }

    pub fn maybe_with_context_vars(
        mut self,
        context_vars: Option<impl IntoIterator<Item = ContextVar>>,
    ) -> Self {
        if let Some(context_vars) = context_vars {
            self.context_vars = context_vars.into_iter().collect();
        }
        self
    }

    pub fn with_tools(mut self, tools: impl IntoIterator<Item = ToolDefinition>) -> Self {
        self.tools = tools.into_iter().collect();
        self
    }

    pub fn maybe_with_tools(
        mut self,
        tools: Option<impl IntoIterator<Item = ToolDefinition>>,
    ) -> Self {
        if let Some(tools) = tools {
            self.tools = tools.into_iter().collect();
        }
        self
    }

    pub fn with_context(mut self, context: impl Into<Value>) -> Self {
        self.context = Some(context.into());
        self
    }

    pub fn maybe_with_context(mut self, context: Option<impl Into<Value>>) -> Self {
        if let Some(context) = context {
            self.context = Some(context.into());
        }
        self
    }

    pub fn to_input(&self) -> ReActStateInput {
        ReActStateInput {
            run_id: self.run_id,
            session_id: self.session_id,
            model_override: self.model_override.clone(),
            capability_override: self.capability_override.clone(),
            messages: self
                .messages
                .iter()
                .map(|msg| msg.to_entity(self.tenant_id.clone(), self.session_id))
                .collect(),
            context_vars: self.context_vars.clone(),
            tools: self.tools.clone(),
            context: self
                .context
                .clone()
                .unwrap_or(Value::Object(Default::default())),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunResponse {
    pub id: Uuid,
    pub tenant_id: String,
    pub session_id: Uuid,
    pub status: RunStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl RunResponse {
    pub fn from_entity(run: &Run) -> Self {
        Self {
            id: run.id,
            tenant_id: run.tenant_id.clone(),
            session_id: run.session_id,
            status: run.status,
            created_at: run.created_at,
            updated_at: run.updated_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindRunParams {
    pub per_page: Option<u64>,
    pub page: Option<String>,
    pub tenant_ids: Option<Vec<String>>,
    pub ids: Option<Vec<Uuid>>,
    pub session_ids: Option<Vec<Uuid>>,
    pub statuses: Option<Vec<RunStatus>>,
    pub created_before: Option<DateTime<Utc>>,
    pub created_after: Option<DateTime<Utc>>,
    pub updated_before: Option<DateTime<Utc>>,
    pub updated_after: Option<DateTime<Utc>>,
}

impl FindRunParams {
    pub fn new() -> Self {
        Self { per_page: Some(10), ..Default::default() }
    }

    pub fn per_page(mut self, per_page: impl Into<u64>) -> Self {
        self.per_page = Some(per_page.into());
        self
    }

    pub fn maybe_per_page(mut self, per_page: Option<impl Into<u64>>) -> Self {
        self.per_page = per_page.map(Into::into);
        self
    }

    pub fn no_limit(mut self) -> Self {
        self.per_page = None;
        self
    }

    pub fn page(mut self, page: impl Into<String>) -> Self {
        self.page = Some(page.into());
        self
    }

    pub fn tenant_ids(mut self, tenant_ids: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.tenant_ids = Some(
            tenant_ids
                .into_iter()
                .map(Into::into)
                .collect(),
        );
        self
    }

    pub fn ids(mut self, ids: impl IntoIterator<Item = impl Into<Uuid>>) -> Self {
        self.ids = Some(
            ids.into_iter()
                .map(Into::into)
                .collect(),
        );
        self
    }

    pub fn session_ids(mut self, session_ids: impl IntoIterator<Item = impl Into<Uuid>>) -> Self {
        self.session_ids = Some(
            session_ids
                .into_iter()
                .map(Into::into)
                .collect(),
        );
        self
    }

    pub fn statuses(mut self, statuses: impl IntoIterator<Item = impl Into<RunStatus>>) -> Self {
        self.statuses = Some(
            statuses
                .into_iter()
                .map(Into::into)
                .collect(),
        );
        self
    }

    pub fn created_before(mut self, created_before: impl Into<DateTime<Utc>>) -> Self {
        self.created_before = Some(created_before.into());
        self
    }

    pub fn created_after(mut self, created_after: impl Into<DateTime<Utc>>) -> Self {
        self.created_after = Some(created_after.into());
        self
    }

    pub fn updated_before(mut self, updated_before: impl Into<DateTime<Utc>>) -> Self {
        self.updated_before = Some(updated_before.into());
        self
    }

    pub fn updated_after(mut self, updated_after: impl Into<DateTime<Utc>>) -> Self {
        self.updated_after = Some(updated_after.into());
        self
    }
}

impl Default for FindRunParams {
    fn default() -> Self {
        Self {
            per_page: Some(10),
            page: None,
            tenant_ids: None,
            ids: None,
            session_ids: None,
            statuses: None,
            created_before: None,
            created_after: None,
            updated_before: None,
            updated_after: None,
        }
    }
}

impl From<FindRunParams> for RepoFindRunParams {
    fn from(params: FindRunParams) -> Self {
        Self {
            per_page: params.per_page,
            page: params.page,
            tenant_ids: params.tenant_ids,
            ids: params.ids,
            session_ids: params.session_ids,
            statuses: params.statuses,
            created_before: params.created_before,
            created_after: params.created_after,
            updated_before: params.updated_before,
            updated_after: params.updated_after,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{
        service::types::message::CreateUserMessageParams,
        types::message::{MediaSource, UserContent},
    };

    #[test]
    fn run_input_preserves_user_content() {
        let content = vec![
            UserContent::text("Describe this image"),
            UserContent::image(MediaSource::Base64("image-data".to_string()), "image/png"),
        ];
        let input = RunParams::new("tenant", Uuid::new_v4())
            .with_messages([CreateMessageParams::User(
                CreateUserMessageParams::from_content(content.clone()),
            )])
            .to_input();

        assert_eq!(
            input.messages[0]
                .as_user()
                .expect("expected user message")
                .content,
            content,
        );
    }
}
