// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use chrono::{DateTime, Utc};
use json_patch::PatchOperation;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
use validator::Validate;

use agentc_agent::types::{capability::CapabilityOverride, tools::ToolDefinition};
use agentc_domain::types::run::RunStatus;
use agentc_model::types::inference::InferenceParams;

use crate::{
    api::dto::v1::message::{CreateMessageRequestDTO, MessageResponseDTO},
    service::types::run::{FindRunParams, RunEvent, RunParams, RunResponse},
    types::{
        context_var::ContextVar,
        event::ReasoningSignatureSubtype,
        model::{ModelConfig, ModelConfigOverride, ModelConfigRetry},
    },
};

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum RunStatusDTO {
    Running,
    Interrupted,
    Completed,
    Failed,
    Cancelled,
}

impl RunStatusDTO {
    pub fn from_status(status: RunStatus) -> Self {
        match status {
            RunStatus::Running => RunStatusDTO::Running,
            RunStatus::Interrupted => RunStatusDTO::Interrupted,
            RunStatus::Completed => RunStatusDTO::Completed,
            RunStatus::Failed => RunStatusDTO::Failed,
            RunStatus::Cancelled => RunStatusDTO::Cancelled,
        }
    }

    pub fn into_status(self) -> RunStatus {
        match self {
            RunStatusDTO::Running => RunStatus::Running,
            RunStatusDTO::Interrupted => RunStatus::Interrupted,
            RunStatusDTO::Completed => RunStatus::Completed,
            RunStatusDTO::Failed => RunStatus::Failed,
            RunStatusDTO::Cancelled => RunStatus::Cancelled,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InferenceParamsDTO {
    pub max_tokens: Option<u64>,
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
    pub top_k: Option<u32>,
    pub stop_sequences: Option<Vec<String>>,
    pub frequency_penalty: Option<f64>,
    pub presence_penalty: Option<f64>,
    pub seed: Option<u64>,
    pub provider_params: Option<serde_json::Value>,
}

impl InferenceParamsDTO {
    pub fn from_response(params: InferenceParams) -> Self {
        Self {
            max_tokens: params.max_tokens,
            temperature: params.temperature,
            top_p: params.top_p,
            top_k: params.top_k,
            stop_sequences: params.stop_sequences,
            frequency_penalty: params.frequency_penalty,
            presence_penalty: params.presence_penalty,
            seed: params.seed,
            provider_params: params.provider_params,
        }
    }

    pub fn to_params(&self) -> InferenceParams {
        InferenceParams {
            max_tokens: self.max_tokens,
            temperature: self.temperature,
            top_p: self.top_p,
            top_k: self.top_k,
            stop_sequences: self.stop_sequences.clone(),
            frequency_penalty: self.frequency_penalty,
            presence_penalty: self.presence_penalty,
            seed: self.seed,
            provider_params: self.provider_params.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct ModelConfigDTO {
    #[serde(rename = "override")]
    pub r#override: Option<ModelConfigOverrideDTO>,
    pub timeout: Option<u64>,
    pub retry: Option<ModelConfigRetryDTO>,
}

impl ModelConfigDTO {
    pub fn from_response(response: ModelConfig) -> Self {
        Self {
            r#override: response
                .r#override
                .map(ModelConfigOverrideDTO::from_response),
            timeout: response.timeout,
            retry: response
                .retry
                .map(ModelConfigRetryDTO::from_response),
        }
    }

    pub fn to_params(&self) -> ModelConfig {
        ModelConfig {
            r#override: self
                .r#override
                .as_ref()
                .map(ModelConfigOverrideDTO::to_params),
            timeout: self.timeout,
            retry: self
                .retry
                .as_ref()
                .map(ModelConfigRetryDTO::to_params),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct ModelConfigOverrideDTO {
    pub provider: Option<String>,
    pub model: Option<String>,
    pub inference_params: Option<InferenceParamsDTO>,
}

impl ModelConfigOverrideDTO {
    pub fn from_response(response: ModelConfigOverride) -> Self {
        Self {
            provider: response.provider,
            model: response.model,
            inference_params: response
                .inference_params
                .map(InferenceParamsDTO::from_response),
        }
    }

    pub fn to_params(&self) -> ModelConfigOverride {
        ModelConfigOverride {
            provider: self.provider.clone(),
            model: self.model.clone(),
            inference_params: self
                .inference_params
                .as_ref()
                .map(InferenceParamsDTO::to_params),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct ModelConfigRetryDTO {
    pub max_attempts: u32,
    pub initial_backoff: u64,
    pub max_backoff: u64,
}

impl ModelConfigRetryDTO {
    pub fn from_response(response: ModelConfigRetry) -> Self {
        Self {
            max_attempts: response.max_attempts,
            initial_backoff: response.initial_backoff,
            max_backoff: response.max_backoff,
        }
    }

    pub fn to_params(&self) -> ModelConfigRetry {
        ModelConfigRetry {
            max_attempts: self.max_attempts,
            initial_backoff: self.initial_backoff,
            max_backoff: self.max_backoff,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "strategy", content = "capabilities", rename_all = "snake_case")]
pub enum CapabilityOverrideDTO {
    Inherit,
    Extend(Vec<String>),
    Replace(Vec<String>),
}

impl CapabilityOverrideDTO {
    pub fn from_response(response: CapabilityOverride) -> Self {
        match response {
            CapabilityOverride::Inherit => CapabilityOverrideDTO::Inherit,
            CapabilityOverride::Extend(capabilities) => CapabilityOverrideDTO::Extend(
                capabilities
                    .into_inner()
                    .into_iter()
                    .map(|c| c.into_string())
                    .collect(),
            ),
            CapabilityOverride::Replace(capabilities) => CapabilityOverrideDTO::Replace(
                capabilities
                    .into_inner()
                    .into_iter()
                    .map(|c| c.into_string())
                    .collect(),
            ),
        }
    }

    pub fn to_params(&self) -> CapabilityOverride {
        match self {
            CapabilityOverrideDTO::Inherit => CapabilityOverride::Inherit,
            CapabilityOverrideDTO::Extend(capabilities) => {
                CapabilityOverride::Extend(capabilities.clone().into())
            }
            CapabilityOverrideDTO::Replace(capabilities) => {
                CapabilityOverride::Replace(capabilities.clone().into())
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct ContextVarDTO {
    pub description: String,
    pub value: String,
}

impl ContextVarDTO {
    pub fn from_response(response: ContextVar) -> Self {
        Self {
            description: response.description,
            value: response.value,
        }
    }

    pub fn to_params(&self) -> ContextVar {
        ContextVar {
            description: self.description.clone(),
            value: self.value.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct ToolDefinitionDTO {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

impl ToolDefinitionDTO {
    pub fn from_response(response: ToolDefinition) -> Self {
        Self {
            name: response.name,
            description: response.description,
            parameters: response.parameters,
        }
    }

    pub fn to_params(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.name.clone(),
            description: self.description.clone(),
            parameters: self.parameters.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct CreateRunRequestDTO {
    #[serde(default = "Uuid::new_v4")]
    pub session_id: Uuid,
    #[serde(default = "Uuid::new_v4")]
    pub run_id: Uuid,
    #[serde(default)]
    pub checkpoint_id: Option<Uuid>,
    #[serde(default)]
    pub resume_payload: Option<Value>,
    #[validate(nested)]
    #[serde(default)]
    pub model: Option<ModelConfigDTO>,
    #[serde(default)]
    pub capability_override: Option<CapabilityOverrideDTO>,
    #[validate(length(min = 1))]
    #[serde(default)]
    pub messages: Vec<CreateMessageRequestDTO>,
    #[validate(nested)]
    #[serde(default)]
    pub context_vars: Vec<ContextVarDTO>,
    #[serde(default)]
    pub context: Option<Value>,
    #[validate(nested)]
    #[serde(default)]
    pub tools: Vec<ToolDefinitionDTO>,
}

impl CreateRunRequestDTO {
    pub fn to_params(&self, tenant_id: impl Into<String>) -> RunParams {
        RunParams {
            tenant_id: tenant_id.into(),
            session_id: self.session_id,
            run_id: self.run_id,
            checkpoint_id: self.checkpoint_id,
            resume_payload: self.resume_payload.clone(),
            model: self
                .model
                .as_ref()
                .map(|m| m.to_params()),
            capability_override: self
                .capability_override
                .as_ref()
                .map(|c| c.to_params()),
            messages: self
                .messages
                .iter()
                .map(|m| m.to_params())
                .collect(),
            context_vars: self
                .context_vars
                .iter()
                .map(|c| c.to_params())
                .collect(),
            tools: self
                .tools
                .iter()
                .map(|t| t.to_params())
                .collect(),
            context: self.context.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct StartRunRequestDTO {
    #[serde(default = "Uuid::new_v4")]
    pub session_id: Uuid,
    #[serde(default = "Uuid::new_v4")]
    pub run_id: Uuid,
    #[serde(default)]
    pub checkpoint_id: Option<Uuid>,
    #[serde(default)]
    pub resume_payload: Option<Value>,
    #[validate(nested)]
    #[serde(default)]
    pub model: Option<ModelConfigDTO>,
    #[serde(default)]
    pub capability_override: Option<CapabilityOverrideDTO>,
    #[validate(length(min = 1))]
    #[serde(default)]
    pub messages: Vec<CreateMessageRequestDTO>,
    #[validate(nested)]
    #[serde(default)]
    pub context_vars: Vec<ContextVarDTO>,
    #[serde(default)]
    pub context: Option<Value>,
    #[validate(nested)]
    #[serde(default)]
    pub tools: Vec<ToolDefinitionDTO>,
}

impl StartRunRequestDTO {
    pub fn to_params(&self, tenant_id: impl Into<String>) -> RunParams {
        RunParams {
            tenant_id: tenant_id.into(),
            session_id: self.session_id,
            run_id: self.run_id,
            checkpoint_id: self.checkpoint_id,
            resume_payload: self.resume_payload.clone(),
            model: self
                .model
                .as_ref()
                .map(|m| m.to_params()),
            capability_override: self
                .capability_override
                .as_ref()
                .map(|c| c.to_params()),
            messages: self
                .messages
                .iter()
                .map(|m| m.to_params())
                .collect(),
            context_vars: self
                .context_vars
                .iter()
                .map(|c| c.to_params())
                .collect(),
            tools: self
                .tools
                .iter()
                .map(|t| t.to_params())
                .collect(),
            context: self.context.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct StartRunResponseDTO {
    pub run_id: Uuid,
    pub session_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RunResponseDTO {
    pub id: Uuid,
    pub session_id: Uuid,
    pub status: RunStatusDTO,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl RunResponseDTO {
    pub fn from_response(response: RunResponse) -> Self {
        Self {
            id: response.id,
            session_id: response.session_id,
            status: RunStatusDTO::from_status(response.status),
            created_at: response.created_at,
            updated_at: response.updated_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, IntoParams)]
pub struct FindRunEndpointParams {
    #[param(minimum = 1, maximum = 100)]
    #[validate(range(min = 1, max = 100))]
    pub per_page: Option<u64>,
    pub page: Option<String>,
    pub ids: Option<Vec<Uuid>>,
    pub session_ids: Option<Vec<Uuid>>,
    pub statuses: Option<Vec<RunStatusDTO>>,
    pub created_before: Option<DateTime<Utc>>,
    pub created_after: Option<DateTime<Utc>>,
    pub updated_before: Option<DateTime<Utc>>,
    pub updated_after: Option<DateTime<Utc>>,
}

impl FindRunEndpointParams {
    pub fn to_params(&self, tenant_id: impl Into<String>) -> FindRunParams {
        FindRunParams {
            per_page: self.per_page,
            page: self.page.clone(),
            ids: self.ids.clone(),
            tenant_ids: Some(vec![tenant_id.into()]),
            session_ids: self.session_ids.clone(),
            statuses: self.statuses.clone().map(|v| {
                v.into_iter()
                    .map(RunStatusDTO::into_status)
                    .collect()
            }),
            created_before: self.created_before,
            created_after: self.created_after,
            updated_before: self.updated_before,
            updated_after: self.updated_after,
        }
    }
}

// #[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
// pub struct StateResponseDTO {
//     pub run_id: Uuid,
//     pub session_id: Uuid,
//     pub model: Option<ModelConfigDTO>,
//     pub capability_override: Option<CapabilityOverrideDTO>,
//     pub messages: Vec<MessageResponseDTO>,
//     pub context_vars: Vec<ContextVarDTO>,
//     pub context: Value,
//     pub tools: Vec<ToolDefinitionDTO>,
// }

// impl StateResponseDTO {
//     pub fn from_response(response: StateResponse) -> Self {
//         Self {
//             run_id: response.run_id,
//             session_id: response.session_id,
//             model: response.model.map(ModelConfigDTO::from_response),
//             capability_override: response.capability_override.map(CapabilityOverrideDTO::from_response),
//             messages: response.messages
//                 .into_iter()
//                 .map(MessageResponseDTO::from_response)
//                 .collect(),
//             context_vars: response.context_vars
//                 .into_iter()
//                 .map(ContextVarDTO::from_response)
//                 .collect(),
//             context: response.context,
//             tools: response.tools
//                 .into_iter()
//                 .map(ToolDefinitionDTO::from_response)
//                 .collect(),
//         }
//     }
// }

// #[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
// pub struct StateUpdateResponseDTO {
//     pub messages: Vec<MessageResponseDTO>,
//     pub context: Vec<PatchOperation>,
// }

// impl StateUpdateResponseDTO {
//     pub fn from_response(response: StateUpdateResponse) -> Self {
//         Self {
//             messages: response.messages
//                 .into_iter()
//                 .map(MessageResponseDTO::from_response)
//                 .collect(),
//             context: response.context,
//         }
//     }

//     pub fn into_patch(self) -> Patch {
//         Patch(
//             (!self.messages.is_empty())
//                 .then(|| {
//                     PatchOperation::Add(AddOperation {
//                         path: "/messages".try_into().expect("invalid patch path"),
//                         value: to_value(self.messages).expect("failed to serialize messages"),
//                     })
//                 })
//                 .into_iter()
//                 .chain(self.context.into_iter().filter_map(|patch_op| match patch_op {
//                     PatchOperation::Add(mut add_op) => {
//                         add_op.path = format!("/context{}", add_op.path).try_into().ok()?;
//                         Some(PatchOperation::Add(add_op))
//                     },
//                     PatchOperation::Remove(mut remove_op) => {
//                         remove_op.path = format!("/context{}", remove_op.path).try_into().ok()?;
//                         Some(PatchOperation::Remove(remove_op))
//                     },
//                     PatchOperation::Replace(mut replace_op) => {
//                         replace_op.path = format!("/context{}", replace_op.path).try_into().ok()?;
//                         Some(PatchOperation::Replace(replace_op))
//                     },
//                     PatchOperation::Move(mut move_op) => {
//                         move_op.from = format!("/context{}", move_op.from).try_into().ok()?;
//                         move_op.path = format!("/context{}", move_op.path).try_into().ok()?;
//                         Some(PatchOperation::Move(move_op))
//                     },
//                     PatchOperation::Copy(mut copy_op) => {
//                         copy_op.from = format!("/context{}", copy_op.from).try_into().ok()?;
//                         copy_op.path = format!("/context{}", copy_op.path).try_into().ok()?;
//                         Some(PatchOperation::Copy(copy_op))
//                     },
//                     PatchOperation::Test(mut test_op) => {
//                         test_op.path = format!("/context{}", test_op.path).try_into().ok()?;
//                         Some(PatchOperation::Test(test_op))
//                     }
//                 }))
//                 .collect()
//         )
//     }
// }

/// DTO for [`ReasoningSignatureSubtype`](crate::types::event::ReasoningSignatureSubtype).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReasoningSignatureSubtypeDTO {
    Message,
    ToolCall,
}

impl From<ReasoningSignatureSubtype> for ReasoningSignatureSubtypeDTO {
    fn from(value: ReasoningSignatureSubtype) -> Self {
        match value {
            ReasoningSignatureSubtype::Message => Self::Message,
            ReasoningSignatureSubtype::ToolCall => Self::ToolCall,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RunEventDTO {
    RunStarted {
        timestamp: f64,
        session_id: Uuid,
        run_id: Uuid,
    },
    RunFinished {
        timestamp: f64,
        session_id: Uuid,
        run_id: Uuid,
        status: RunStatusDTO,
        interrupt_payload: Option<Value>,
        // result: Option<StateResponseDTO>,
        result: Option<Value>,
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
        subtype: ReasoningSignatureSubtypeDTO,
        entity_id: String,
        value: String,
    },
    StateSnapshot {
        timestamp: f64,
        // state: StateResponseDTO,
        state: Value,
    },
    StateDelta {
        timestamp: f64,
        delta: Vec<PatchOperation>,
    },
    MessagesSnapshot {
        timestamp: f64,
        messages: Vec<MessageResponseDTO>,
    },
}

impl RunEventDTO {
    pub fn from_event(event: RunEvent) -> Self {
        match event {
            RunEvent::RunStarted { timestamp, session_id, run_id } => {
                Self::RunStarted { timestamp, session_id, run_id }
            }
            RunEvent::RunFinished {
                timestamp,
                session_id,
                run_id,
                status,
                interrupt_payload,
                result,
            } => Self::RunFinished {
                timestamp,
                session_id,
                run_id,
                status: RunStatusDTO::from_status(status),
                interrupt_payload,
                // result: result.map(StateResponseDTO::from_response),
                result: result.map(|r| r.context),
            },
            RunEvent::RunError {
                timestamp,
                session_id,
                run_id,
                error,
                code,
            } => Self::RunError {
                timestamp,
                session_id,
                run_id,
                error,
                code,
            },
            RunEvent::TextMessageStart { timestamp, message_id } => {
                Self::TextMessageStart { timestamp, message_id }
            }
            RunEvent::TextMessageEnd { timestamp, message_id } => {
                Self::TextMessageEnd { timestamp, message_id }
            }
            RunEvent::TextMessageContent { timestamp, message_id, delta } => {
                Self::TextMessageContent { timestamp, message_id, delta }
            }
            RunEvent::ToolCallStart { timestamp, tool_call_id, tool_name } => {
                Self::ToolCallStart { timestamp, tool_call_id, tool_name }
            }
            RunEvent::ToolCallEnd { timestamp, tool_call_id } => {
                Self::ToolCallEnd { timestamp, tool_call_id }
            }
            RunEvent::ToolCallArgs { timestamp, tool_call_id, delta } => {
                Self::ToolCallArgs { timestamp, tool_call_id, delta }
            }
            RunEvent::ToolCallError {
                timestamp,
                tool_call_id,
                message_id,
                error,
                code,
            } => Self::ToolCallError {
                timestamp,
                tool_call_id,
                message_id,
                error,
                code,
            },
            RunEvent::ToolCallResult {
                timestamp,
                tool_call_id,
                message_id,
                content,
            } => Self::ToolCallResult {
                timestamp,
                tool_call_id,
                message_id,
                content,
            },
            RunEvent::ActivityDelta {
                timestamp,
                tool_call_id,
                activity_type,
                patch,
            } => Self::ActivityDelta {
                timestamp,
                tool_call_id,
                activity_type,
                patch,
            },
            RunEvent::ReasoningStart { timestamp, message_id } => {
                Self::ReasoningStart { timestamp, message_id }
            }
            RunEvent::ReasoningEnd { timestamp, message_id } => {
                Self::ReasoningEnd { timestamp, message_id }
            }
            RunEvent::ReasoningMessageStart { timestamp, message_id } => {
                Self::ReasoningMessageStart { timestamp, message_id }
            }
            RunEvent::ReasoningMessageContent { timestamp, message_id, delta } => {
                Self::ReasoningMessageContent { timestamp, message_id, delta }
            }
            RunEvent::ReasoningMessageEnd { timestamp, message_id } => {
                Self::ReasoningMessageEnd { timestamp, message_id }
            }
            RunEvent::ReasoningSignature {
                timestamp,
                message_id,
                subtype,
                entity_id,
                value,
            } => Self::ReasoningSignature {
                timestamp,
                message_id,
                subtype: subtype.into(),
                entity_id,
                value,
            },
            RunEvent::StateSnapshot { timestamp, state } => Self::StateSnapshot {
                timestamp,
                // state: StateResponseDTO::from_response(state),
                state: state.context,
            },
            RunEvent::StateDelta { timestamp, delta } => Self::StateDelta {
                timestamp,
                // delta: StateUpdateResponseDTO::from_response(delta)
                //     .into_patch(),
                delta: delta.context,
            },
            RunEvent::MessagesSnapshot { timestamp, messages } => Self::MessagesSnapshot {
                timestamp,
                messages: messages
                    .into_iter()
                    .map(MessageResponseDTO::from_response)
                    .collect(),
            },
        }
    }
}
