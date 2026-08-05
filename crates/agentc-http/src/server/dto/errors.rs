// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use axum::{http::StatusCode, response::IntoResponse, response::Json, response::Response};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(untagged)]
pub enum ErrorResponseDTO {
    Generic {
        code: u32,
        message: String,
    },
    Validation {
        code: u32,
        fields: Vec<ValidationErrorFieldDTO>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ValidationErrorFieldDTO {
    pub field: String,
    pub errors: Vec<ValidationErrorFieldDetailDTO>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ValidationErrorFieldDetailDTO {
    pub code: String,
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl IntoResponse for ErrorResponseDTO {
    fn into_response(self) -> Response {
        let status = match self {
            ErrorResponseDTO::Generic { code, .. } => StatusCode::from_u16((code / 1000) as u16)
                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            ErrorResponseDTO::Validation { code, .. } => {
                StatusCode::from_u16((code / 1000) as u16).unwrap_or(StatusCode::BAD_REQUEST)
            }
        };

        (status, Json(self)).into_response()
    }
}
