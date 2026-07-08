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
};

use crate::{
    api::state::ReActApiState,
    api::dto::v1::message::{
        CreateMessageRequestDTO, FindMessageEndpointParams, MessageResponseDTO,
    },
    service::operations::{message::MessageOperations, session::SessionOperations},
};

pub fn router() -> OpenApiRouter<ReActApiState> {
    OpenApiRouter::new()
        .routes(routes!(find_messages_endpoint))
        .routes(routes!(create_message_endpoint))
        .routes(routes!(get_message_endpoint))
        .routes(routes!(delete_message_endpoint))
}

/// Find messages for a session
#[utoipa::path(
    get,
    path = "/sessions/{session_id}/messages",
    operation_id = "find_messages",
    tag = "messages",
    description = "Find messages for a session with optional filtering and pagination.",
    params(
        FindMessageEndpointParams,
        ("session_id" = Uuid, Path, description = "The ID of the session"),
        ("X-Tenant_id" = Option<TenantIdHeader>, Header, description = "The ID of the tenant"),
    ),
    responses(
        (status = 200, description = "Messages retrieved successfully", body = PaginatedResponseDTO<MessageResponseDTO>),
        (status = 400, description = "Bad request", body = ErrorResponseDTO),
        (status = 404, description = "Session not found", body = ErrorResponseDTO),
        (status = 500, description = "Internal server error", body = ErrorResponseDTO)
    ),
)]
async fn find_messages_endpoint(
    State(state): State<ReActApiState>,
    Path(session_id): Path<Uuid>,
    Query(params): Query<FindMessageEndpointParams>,
    tenant_id: Option<TenantIdHeader>,
) -> Response {
    if let Err(err) = params.validate() {
        return ErrorResponseDTO::from(ApiError::from(err)).into_response();
    }

    let tenant_id = tenant_id.map_or(
        state.default_tenant_id.clone().into_inner(),
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
        .find_messages(params.to_params(tenant_id, session_id))
        .await
    {
        Ok(response) => {
            PaginatedResponseDTO::from_result(response, MessageResponseDTO::from_response)
                .into_response()
        }
        Err(err) => ErrorResponseDTO::from(ApiError::from(err)).into_response(),
    }
}

/// Create a new message in a session
#[utoipa::path(
    post,
    path = "/sessions/{session_id}/messages",
    operation_id = "create_message",
    tag = "messages",
    description = "Create a new message in a session outside of a run.",
    params(
        ("session_id" = Uuid, Path, description = "The ID of the session"),
        ("X-Tenant_id" = Option<TenantIdHeader>, Header, description = "The ID of the tenant"),
    ),
    request_body = CreateMessageRequestDTO,
    responses(
        (status = 201, description = "Message created successfully", body = MessageResponseDTO),
        (status = 400, description = "Bad request", body = ErrorResponseDTO),
        (status = 404, description = "Session not found", body = ErrorResponseDTO),
        (status = 500, description = "Internal server error", body = ErrorResponseDTO)
    ),
)]
async fn create_message_endpoint(
    State(state): State<ReActApiState>,
    Path(session_id): Path<Uuid>,
    tenant_id: Option<TenantIdHeader>,
    Json(payload): Json<CreateMessageRequestDTO>,
) -> Response {
    if let Err(err) = payload.validate() {
        return ErrorResponseDTO::from(ApiError::from(err)).into_response();
    }

    let tenant_id = tenant_id.map_or(
        state.default_tenant_id.clone().into_inner(),
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
        .create_message(&tenant_id, session_id, payload.to_params())
        .await
    {
        Ok(response) => {
            (StatusCode::CREATED, Json(MessageResponseDTO::from_response(response))).into_response()
        }
        Err(err) => ErrorResponseDTO::from(ApiError::from(err)).into_response(),
    }
}

/// Get a message by ID
#[utoipa::path(
    get,
    path = "/messages/{message_id}",
    operation_id = "get_message",
    tag = "messages",
    description = "Get a message by its ID.",
    params(
        ("message_id" = Uuid, Path, description = "The ID of the message"),
        ("X-Tenant_id" = Option<TenantIdHeader>, Header, description = "The ID of the tenant"),
    ),
    responses(
        (status = 200, description = "Message retrieved successfully", body = MessageResponseDTO),
        (status = 400, description = "Bad request", body = ErrorResponseDTO),
        (status = 404, description = "Message not found", body = ErrorResponseDTO),
        (status = 500, description = "Internal server error", body = ErrorResponseDTO)
    ),
)]
async fn get_message_endpoint(
    State(state): State<ReActApiState>,
    Path(message_id): Path<Uuid>,
    tenant_id: Option<TenantIdHeader>,
) -> Response {
    match state
        .service
        .get_message(
            &tenant_id.map_or(
                state.default_tenant_id.into_inner(),
                TenantIdHeader::into_inner,
            ),
            message_id,
        )
        .await
    {
        Ok(response) => {
            (StatusCode::OK, Json(MessageResponseDTO::from_response(response))).into_response()
        }
        Err(err) => ErrorResponseDTO::from(ApiError::from(err)).into_response(),
    }
}

/// Delete a message by ID
#[utoipa::path(
    delete,
    path = "/messages/{message_id}",
    operation_id = "delete_message",
    tag = "messages",
    description = "Delete a message by its ID.",
    params(
        ("message_id" = Uuid, Path, description = "The ID of the message"),
        ("X-Tenant_id" = Option<TenantIdHeader>, Header, description = "The ID of the tenant"),
    ),
    responses(
        (status = 204, description = "Message deleted successfully"),
        (status = 400, description = "Bad request", body = ErrorResponseDTO),
        (status = 404, description = "Message not found", body = ErrorResponseDTO),
        (status = 500, description = "Internal server error", body = ErrorResponseDTO)
    ),
)]
async fn delete_message_endpoint(
    State(state): State<ReActApiState>,
    Path(message_id): Path<Uuid>,
    tenant_id: Option<TenantIdHeader>,
) -> Response {
    match state
        .service
        .delete_messages(
            &tenant_id.map_or(
                state.default_tenant_id.into_inner(),
                TenantIdHeader::into_inner,
            ),
            &[message_id],
        )
        .await
    {
        Ok(()) => NoContent.into_response(),
        Err(err) => ErrorResponseDTO::from(ApiError::from(err)).into_response(),
    }
}
