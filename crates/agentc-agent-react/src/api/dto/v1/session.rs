// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
use validator::Validate;

use crate::service::types::session::{CreateSessionParams, FindSessionParams, SessionResponse};

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
#[serde(default)]
pub struct CreateSessionRequestDTO {
    pub id: Uuid,
}

impl CreateSessionRequestDTO {
    pub fn to_params(&self, tenant_id: impl Into<String>) -> CreateSessionParams {
        CreateSessionParams { tenant_id: tenant_id.into(), id: self.id }
    }
}

impl Default for CreateSessionRequestDTO {
    fn default() -> Self {
        Self { id: Uuid::new_v4() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SessionResponseDTO {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl SessionResponseDTO {
    pub fn from_response(response: SessionResponse) -> Self {
        Self {
            id: response.id,
            created_at: response.created_at,
            updated_at: response.updated_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, IntoParams)]
pub struct FindSessionEndpointParams {
    #[param(minimum = 1, maximum = 100)]
    #[validate(range(min = 1, max = 100))]
    pub per_page: Option<u64>,
    pub page: Option<String>,
    pub ids: Option<Vec<Uuid>>,
    pub created_before: Option<DateTime<Utc>>,
    pub created_after: Option<DateTime<Utc>>,
    pub updated_before: Option<DateTime<Utc>>,
    pub updated_after: Option<DateTime<Utc>>,
}

impl FindSessionEndpointParams {
    pub fn to_params(self, tenant_id: impl Into<String>) -> FindSessionParams {
        FindSessionParams {
            per_page: self.per_page,
            page: self.page,
            tenant_ids: Some(vec![tenant_id.into()]),
            ids: self.ids,
            created_before: self.created_before,
            created_after: self.created_after,
            updated_before: self.updated_before,
            updated_after: self.updated_after,
        }
    }
}
