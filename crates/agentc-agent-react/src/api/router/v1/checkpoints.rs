// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use axum::{
    extract::State,
    response::{IntoResponse, Response},
};
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;
use validator::Validate;

use agentc_http::{
    dto::{errors::ErrorResponseDTO, page::PaginatedResponseDTO},
    errors::ApiError,
    extractors::{Path, Query, TenantIdHeader},
};

use crate::{
    api::dto::v1::checkpoint::{CheckpointResponseDTO, FindCheckpointEndpointParams},
    api::state::ReActApiState,
    service::operations::{checkpoint::CheckpointOperations, session::SessionOperations},
};

pub fn router() -> OpenApiRouter<ReActApiState> {
    OpenApiRouter::new().routes(routes!(find_checkpoints_endpoint))
}

/// Find checkpoints for a session
#[utoipa::path(
    get,
    path = "/sessions/{session_id}/checkpoints",
    operation_id = "find_checkpoints",
    tag = "checkpoints",
    description = "Find checkpoints for a session with optional filtering and pagination.",
    params(
        FindCheckpointEndpointParams,
        ("session_id" = Uuid, Path, description = "The ID of the session"),
        ("X-Tenant_id" = Option<TenantIdHeader>, Header, description = "The ID of the tenant"),
    ),
    responses(
        (status = 200, description = "Checkpoints retrieved successfully", body = PaginatedResponseDTO<CheckpointResponseDTO>),
        (status = 400, description = "Bad request", body = ErrorResponseDTO),
        (status = 404, description = "Session not found", body = ErrorResponseDTO),
        (status = 500, description = "Internal server error", body = ErrorResponseDTO)
    ),
)]
async fn find_checkpoints_endpoint(
    State(state): State<ReActApiState>,
    Path(session_id): Path<Uuid>,
    Query(params): Query<FindCheckpointEndpointParams>,
    tenant_id: Option<TenantIdHeader>,
) -> Response {
    if let Err(err) = params.validate() {
        return ErrorResponseDTO::from(ApiError::from(err)).into_response();
    }

    let tenant_id = tenant_id.map_or(
        state
            .default_tenant_id
            .clone()
            .into_inner(),
        TenantIdHeader::into_inner,
    );

    if let Err(err) = state
        .service
        .get_session(&tenant_id, session_id)
        .await
    {
        return ErrorResponseDTO::from(ApiError::from(err)).into_response();
    }

    match state
        .service
        .find_checkpoints(params.to_params(tenant_id, session_id))
        .await
    {
        Ok(response) => {
            PaginatedResponseDTO::from_result(response, CheckpointResponseDTO::from_response)
                .into_response()
        }
        Err(err) => ErrorResponseDTO::from(ApiError::from(err)).into_response(),
    }
}
