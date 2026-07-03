// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use axum::{
    extract::State,
    http::StatusCode,
    response::{
        IntoResponse, Response,
        sse::{Event, KeepAlive, Sse},
    },
};
use futures::stream::StreamExt;
use std::{convert::Infallible, time::Duration};
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
    api::dto::v1::run::{CreateRunRequestDTO, FindRunEndpointParams, RunEventDTO, RunResponseDTO},
    service::{
        ApplicationService,
        operations::{run::RunOperations, session::SessionOperations},
    },
};

pub fn router() -> OpenApiRouter<ApiState<ApplicationService>> {
    OpenApiRouter::new()
        .routes(routes!(find_runs_endpoint))
        .routes(routes!(get_run_endpoint))
        .routes(routes!(create_run_endpoint))
}

/// Find runs for a session
#[utoipa::path(
    get,
    path = "/sessions/{session_id}/runs",
    operation_id = "find_runs",
    tag = "runs",
    description = "Find runs for a session with optional filtering and pagination.",
    params(
        FindRunEndpointParams,
        ("session_id" = Uuid, Path, description = "The ID of the session"),
        ("X-Tenant_id" = Option<TenantIdHeader>, Header, description = "The ID of the tenant"),
    ),
    responses(
        (status = 200, description = "Runs retrieved successfully", body = PaginatedResponseDTO<RunResponseDTO>),
        (status = 400, description = "Bad request", body = ErrorResponseDTO),
        (status = 404, description = "Session not found", body = ErrorResponseDTO),
        (status = 500, description = "Internal server error", body = ErrorResponseDTO)
    ),
)]
async fn find_runs_endpoint(
    State(state): State<ApiState<ApplicationService>>,
    Path(session_id): Path<Uuid>,
    Query(params): Query<FindRunEndpointParams>,
    tenant_id: Option<TenantIdHeader>,
) -> Response {
    if let Err(err) = params.validate() {
        return ErrorResponseDTO::from(ApiError::from(err)).into_response();
    }

    let tenant_id = tenant_id.map_or(state.default_tenant_id.clone(), TenantIdHeader::into_inner);

    if let Err(err) = state
        .get_session(&tenant_id, session_id)
        .await
    {
        return ErrorResponseDTO::from(ApiError::from(err)).into_response();
    }

    match state
        .find_runs(params.to_params(tenant_id))
        .await
    {
        Ok(response) => PaginatedResponseDTO::from_result(response, RunResponseDTO::from_response)
            .into_response(),
        Err(err) => ErrorResponseDTO::from(ApiError::from(err)).into_response(),
    }
}

/// Get a run by ID
#[utoipa::path(
    get,
    path = "/runs/{run_id}",
    operation_id = "get_run",
    tag = "runs",
    description = "Get a run by ID.",
    params(
        ("run_id" = Uuid, Path, description = "The ID of the run"),
        ("X-Tenant_id" = Option<TenantIdHeader>, Header, description = "The ID of the tenant"),
    ),
    responses(
        (status = 200, description = "Run retrieved successfully", body = RunResponseDTO),
        (status = 400, description = "Bad request", body = ErrorResponseDTO),
        (status = 404, description = "Run not found", body = ErrorResponseDTO),
        (status = 500, description = "Internal server error", body = ErrorResponseDTO)
    ),
)]
async fn get_run_endpoint(
    State(state): State<ApiState<ApplicationService>>,
    Path(run_id): Path<Uuid>,
    tenant_id: Option<TenantIdHeader>,
) -> Response {
    match state
        .get_run(
            &tenant_id.map_or(state.default_tenant_id.clone(), TenantIdHeader::into_inner),
            run_id,
        )
        .await
    {
        Ok(response) => {
            (StatusCode::OK, Json(RunResponseDTO::from_response(response))).into_response()
        }
        Err(err) => ErrorResponseDTO::from(ApiError::from(err)).into_response(),
    }
}

/// Create a new run
#[utoipa::path(
    post,
    path = "/runs",
    operation_id = "create_run",
    tag = "runs",
    description = "Create a new run and stream its events back as Server-Sent Events (SSE).",
    params(
        ("X-Tenant_id" = Option<TenantIdHeader>, Header, description = "The ID of the tenant"),
    ),
    request_body = CreateRunRequestDTO,
    responses(
        (status = 200, description = "A stream of events", content_type = "text/event-stream"),
        (status = 400, description = "Bad request", body = ErrorResponseDTO),
        (status = 404, description = "Session not found", body = ErrorResponseDTO),
        (status = 500, description = "Internal server error", body = ErrorResponseDTO)
    ),
)]
async fn create_run_endpoint(
    State(state): State<ApiState<ApplicationService>>,
    tenant_id: Option<TenantIdHeader>,
    Json(payload): Json<CreateRunRequestDTO>,
) -> Response {
    if let Err(err) = payload.validate() {
        return ErrorResponseDTO::from(ApiError::from(err)).into_response();
    }

    match state
        .run(payload.to_params(
            tenant_id.map_or(state.default_tenant_id.clone(), TenantIdHeader::into_inner),
        ))
        .await
    {
        Ok((stream, _)) => Sse::new(stream.map(|event| {
            Ok::<_, Infallible>(
                Event::default()
                    .event(event.kind())
                    .json_data(RunEventDTO::from_event(event))
                    .expect("failed to serialize event data"),
            )
        }))
        .keep_alive(
            KeepAlive::new()
                .interval(Duration::from_secs(15))
                .text("keep-alive"),
        )
        .into_response(),
        Err(err) => ErrorResponseDTO::from(ApiError::from(err)).into_response(),
    }
}
