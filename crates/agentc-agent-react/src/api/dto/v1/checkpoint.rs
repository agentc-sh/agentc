// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
use validator::Validate;

use agentc_domain::types::CheckpointReason;

use crate::{
    api::dto::v1::run::RunStatusDTO,
    service::types::checkpoint::{CheckpointResponse, FindCheckpointParams},
};

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CheckpointReasonDTO {
    Input,
    Step,
    Interrupt,
    Finish,
}

impl CheckpointReasonDTO {
    pub fn from_reason(reason: CheckpointReason) -> Self {
        match reason {
            CheckpointReason::Input => CheckpointReasonDTO::Input,
            CheckpointReason::Step => CheckpointReasonDTO::Step,
            CheckpointReason::Interrupt => CheckpointReasonDTO::Interrupt,
            CheckpointReason::Finish => CheckpointReasonDTO::Finish,
        }
    }

    pub fn into_reason(self) -> CheckpointReason {
        match self {
            CheckpointReasonDTO::Input => CheckpointReason::Input,
            CheckpointReasonDTO::Step => CheckpointReason::Step,
            CheckpointReasonDTO::Interrupt => CheckpointReason::Interrupt,
            CheckpointReasonDTO::Finish => CheckpointReason::Finish,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CheckpointResponseDTO {
    pub id: Uuid,
    pub session_id: Uuid,
    pub run_id: Uuid,
    pub node: String,
    pub status: RunStatusDTO,
    pub reason: CheckpointReasonDTO,
    pub parent_checkpoint_id: Option<Uuid>,
    pub metadata: Option<Value>,
    pub created_at: DateTime<Utc>,
}

impl CheckpointResponseDTO {
    pub fn from_response(response: CheckpointResponse) -> Self {
        Self {
            id: response.id,
            session_id: response.session_id,
            run_id: response.run_id,
            node: response.node,
            status: RunStatusDTO::from_status(response.status),
            reason: CheckpointReasonDTO::from_reason(response.reason),
            parent_checkpoint_id: response.parent_checkpoint_id,
            metadata: response.metadata,
            created_at: response.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, IntoParams)]
pub struct FindCheckpointEndpointParams {
    #[param(minimum = 1, maximum = 100)]
    #[validate(range(min = 1, max = 100))]
    pub per_page: Option<u64>,
    pub page: Option<String>,
    pub ids: Option<Vec<Uuid>>,
    pub run_ids: Option<Vec<Uuid>>,
    pub reasons: Option<Vec<CheckpointReasonDTO>>,
    pub statuses: Option<Vec<RunStatusDTO>>,
    pub created_before: Option<DateTime<Utc>>,
    pub created_after: Option<DateTime<Utc>>,
}

impl FindCheckpointEndpointParams {
    pub fn to_params(
        &self,
        tenant_id: impl Into<String>,
        session_id: impl Into<Uuid>,
    ) -> FindCheckpointParams {
        FindCheckpointParams {
            per_page: self.per_page,
            page: self.page.clone(),
            tenant_ids: Some(vec![tenant_id.into()]),
            ids: self.ids.clone(),
            session_ids: Some(vec![session_id.into()]),
            run_ids: self.run_ids.clone(),
            reasons: self.reasons.clone().map(|reasons| {
                reasons
                    .into_iter()
                    .map(CheckpointReasonDTO::into_reason)
                    .collect()
            }),
            statuses: self.statuses.clone().map(|statuses| {
                statuses
                    .into_iter()
                    .map(RunStatusDTO::into_status)
                    .collect()
            }),
            created_before: self.created_before,
            created_after: self.created_after,
        }
    }
}
