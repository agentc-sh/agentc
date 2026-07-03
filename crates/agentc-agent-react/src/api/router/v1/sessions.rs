// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, NoContent, Response},
};
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;
use validator::Validate;

use agentc_http::{
    dto::{errors::ErrorResponseDTO, page::PaginatedResponseDTO},
    errors::ApiError,
    extractors::{Json, Path, Query, TenantIdHeader},
    state::ApiState,
};

use crate::{
    api::dto::v1::session::{
        CreateSessionRequestDTO, FindSessionEndpointParams, SessionResponseDTO,
    },
    service::{ApplicationService, operations::session::SessionOperations},
};

pub fn router() -> OpenApiRouter<ApiState<ApplicationService>> {
    OpenApiRouter::new()
        .routes(routes!(find_sessions_endpoint))
        .routes(routes!(create_session_endpoint))
        .routes(routes!(get_session_endpoint))
        .routes(routes!(delete_session_endpoint))
}

/// Find sessions
#[utoipa::path(
    get,
    path = "/sessions",
    operation_id = "find_sessions",
    tag = "sessions",
    description = "Find sessions with optional filtering and pagination.",
    params(
        FindSessionEndpointParams,
        ("X-Tenant_id" = Option<TenantIdHeader>, Header, description = "The ID of the tenant"),
    ),
    responses(
        (status = 200, description = "Sessions retrieved successfully", body = PaginatedResponseDTO<SessionResponseDTO>),
        (status = 400, description = "Bad request", body = ErrorResponseDTO),
        (status = 500, description = "Internal server error", body = ErrorResponseDTO)
    ),
)]
async fn find_sessions_endpoint(
    State(state): State<ApiState<ApplicationService>>,
    Query(params): Query<FindSessionEndpointParams>,
    tenant_id: Option<TenantIdHeader>,
) -> Response {
    if let Err(err) = params.validate() {
        return ErrorResponseDTO::from(ApiError::from(err)).into_response();
    }

    match state
        .find_sessions(params.to_params(
            tenant_id.map_or(state.default_tenant_id.clone(), TenantIdHeader::into_inner),
        ))
        .await
    {
        Ok(response) => {
            PaginatedResponseDTO::from_result(response, SessionResponseDTO::from_response)
                .into_response()
        }
        Err(err) => ErrorResponseDTO::from(ApiError::from(err)).into_response(),
    }
}

/// Create a new session
#[utoipa::path(
    post,
    path = "/sessions",
    operation_id = "create_session",
    tag = "sessions",
    description = "Create a new session with the specified parameters.",
    request_body = CreateSessionRequestDTO,
    params(
        ("X-Tenant_id" = Option<TenantIdHeader>, Header, description = "The ID of the tenant"),
    ),
    responses(
        (status = 201, description = "Session created successfully", body = SessionResponseDTO),
        (status = 400, description = "Bad request", body = ErrorResponseDTO),
        (status = 422, description = "Validation error", body = ErrorResponseDTO),
        (status = 500, description = "Internal server error", body = ErrorResponseDTO)
    )
)]
async fn create_session_endpoint(
    State(state): State<ApiState<ApplicationService>>,
    tenant_id: Option<TenantIdHeader>,
    Json(payload): Json<CreateSessionRequestDTO>,
) -> Response {
    if let Err(err) = payload.validate() {
        return ErrorResponseDTO::from(ApiError::from(err)).into_response();
    }

    match state
        .create_session(payload.to_params(
            tenant_id.map_or(state.default_tenant_id.clone(), TenantIdHeader::into_inner),
        ))
        .await
    {
        Ok(response) => {
            (StatusCode::CREATED, Json(SessionResponseDTO::from_response(response))).into_response()
        }
        Err(err) => ErrorResponseDTO::from(ApiError::from(err)).into_response(),
    }
}

/// Get a session by ID
#[utoipa::path(
    get,
    path = "/sessions/{session_id}",
    operation_id = "get_session",
    tag = "sessions",
    description = "Get a session by its ID.",
    params(
        ("session_id" = Uuid, Path, description = "The ID of the session"),
        ("X-Tenant_id" = Option<TenantIdHeader>, Header, description = "The ID of the tenant"),
    ),
    responses(
        (status = 200, description = "Session retrieved successfully", body = SessionResponseDTO),
        (status = 400, description = "Bad request", body = ErrorResponseDTO),
        (status = 404, description = "Session not found", body = ErrorResponseDTO),
        (status = 500, description = "Internal server error", body = ErrorResponseDTO)
    )
)]
async fn get_session_endpoint(
    State(state): State<ApiState<ApplicationService>>,
    Path(session_id): Path<Uuid>,
    tenant_id: Option<TenantIdHeader>,
) -> Response {
    match state
        .get_session(
            &tenant_id.map_or(state.default_tenant_id.clone(), TenantIdHeader::into_inner),
            session_id,
        )
        .await
    {
        Ok(response) => {
            (StatusCode::OK, Json(SessionResponseDTO::from_response(response))).into_response()
        }
        Err(err) => ErrorResponseDTO::from(ApiError::from(err)).into_response(),
    }
}

/// Delete a session by ID
#[utoipa::path(
    delete,
    path = "/sessions/{session_id}",
    operation_id = "delete_session",
    tag = "sessions",
    description = "Delete a session by its ID.",
    params(
        ("session_id" = Uuid, Path, description = "The ID of the session"),
        ("X-Tenant_id" = Option<TenantIdHeader>, Header, description = "The ID of the tenant"),
    ),
    responses(
        (status = 204, description = "Session deleted successfully"),
        (status = 400, description = "Bad request", body = ErrorResponseDTO),
        (status = 404, description = "Session not found", body = ErrorResponseDTO),
        (status = 500, description = "Internal server error", body = ErrorResponseDTO)
    ),
)]
async fn delete_session_endpoint(
    State(state): State<ApiState<ApplicationService>>,
    Path(session_id): Path<Uuid>,
    tenant_id: Option<TenantIdHeader>,
) -> Response {
    match state
        .delete_sessions(
            &tenant_id.map_or(state.default_tenant_id.clone(), TenantIdHeader::into_inner),
            &[session_id],
        )
        .await
    {
        Ok(_) => NoContent.into_response(),
        Err(err) => ErrorResponseDTO::from(ApiError::from(err)).into_response(),
    }
}
