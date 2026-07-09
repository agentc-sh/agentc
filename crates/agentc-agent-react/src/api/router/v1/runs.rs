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
use async_stream::try_stream;
use futures::stream::{BoxStream, StreamExt};
use jobq::{
    Error as JobQueueError,
    JobStreamOptions,
    StreamTask,
};
use std::{
    convert::Infallible,
    time::Duration,
};
use subway::Bus;
use tokio_util::sync::CancellationToken;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;
use validator::Validate;

use agentc_domain::types::RunStatus;
use agentc_http::{
    dto::{errors::ErrorResponseDTO, page::PaginatedResponseDTO},
    errors::ApiError,
    extractors::{Json, Path, Query, TenantIdHeader},
    stream::CancelOnDropStream,
};

use crate::{
    api::dto::v1::run::{
        CreateRunRequestDTO,
        FindRunEndpointParams,
        RunEventDTO,
        RunResponseDTO,
        StartRunRequestDTO,
        StartRunResponseDTO,
    },
    api::state::ReActApiState,
    service::{
        ApplicationService,
        errors::ServiceError,
        operations::{run::RunOperations, session::SessionOperations},
        types::run::{
            RunEvent,
            RunParams,
        },
    },
};

struct RunStreamTask {
    service: ApplicationService,
    params: RunParams,
    disconnect: CancellationToken,
    bus: Bus,
}

impl RunStreamTask {
    fn new(
        service: ApplicationService,
        params: RunParams,
        disconnect: CancellationToken,
        bus: Bus,
    ) -> Self {
        Self {
            service,
            params,
            disconnect,
            bus,
        }
    }
}

impl StreamTask for RunStreamTask {
    type Item = RunEvent;
    type Error = ServiceError;

    fn execute(&self) -> BoxStream<'_, Result<Self::Item, Self::Error>> {
        Box::pin(try_stream! {
            let tenant_id = self.params.tenant_id.clone();
            let run_id = self.params.run_id;

            let topic = self.bus
                .topic::<RunEvent>(&format!("run:{tenant_id}:{run_id}"));

            let mut stream = self.service
                .run(self.params.clone())
                .await?;

            loop {
                match tokio::select! {
                    _ = self.disconnect.cancelled() => self.service
                        .cancel_run(&tenant_id, run_id)
                        .await
                        .map(|_| None),
                    event = stream.next() => Ok(event),
                }? {
                    Some(event) => {
                        let _ = topic.publish(&event).await;

                        yield event;
                    }
                    None => break,
                }
            }
        })
    }
}

pub fn router() -> OpenApiRouter<ReActApiState> {
    OpenApiRouter::new()
        .routes(routes!(find_runs_endpoint))
        .routes(routes!(get_run_endpoint))
        .routes(routes!(create_run_endpoint))
        .routes(routes!(start_run_endpoint))
        .routes(routes!(cancel_run_endpoint))
        .routes(routes!(reattach_run_endpoint))
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
    State(state): State<ReActApiState>,
    Path(session_id): Path<Uuid>,
    Query(params): Query<FindRunEndpointParams>,
    tenant_id: Option<TenantIdHeader>,
) -> Response
{
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
        return ErrorResponseDTO::from(ApiError::from(err))
            .into_response();
    }

    match state
        .service
        .find_runs(params.to_params(tenant_id))
        .await
    {
        Ok(response) => PaginatedResponseDTO::from_result(response, RunResponseDTO::from_response)
            .into_response(),
        Err(err) => ErrorResponseDTO::from(ApiError::from(err))
            .into_response(),
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
    State(state): State<ReActApiState>,
    Path(run_id): Path<Uuid>,
    tenant_id: Option<TenantIdHeader>,
) -> Response
{
    match state
        .service
        .get_run(
            &tenant_id.map_or(
                state.default_tenant_id.into_inner(),
                TenantIdHeader::into_inner,
            ),
            run_id,
        )
        .await
    {
        Ok(response) => {
            (StatusCode::OK, Json(RunResponseDTO::from_response(response)))
                .into_response()
        }
        Err(err) => ErrorResponseDTO::from(ApiError::from(err))
            .into_response(),
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
    State(state): State<ReActApiState>,
    tenant_id: Option<TenantIdHeader>,
    Json(payload): Json<CreateRunRequestDTO>,
) -> Response
{
    if let Err(err) = payload.validate() {
        return ErrorResponseDTO::from(ApiError::from(err))
            .into_response();
    }

    let disconnect = CancellationToken::new();

    match state
        .task_queue
        .enqueue_stream(JobStreamOptions::new(RunStreamTask::new(
            (*state.service).clone(),
            payload.to_params(
                tenant_id.map_or(
                    state.default_tenant_id.into_inner(),
                    TenantIdHeader::into_inner,
                ),
            ),
            disconnect.clone(),
            state.bus.clone(),
        )))
        .await
    {
        Ok(stream) => Sse::new(CancelOnDropStream::new(stream, disconnect).map(|result| {
            match result {
                Ok(event) => Ok::<_, Infallible>(
                    Event::default()
                        .event(event.kind())
                        .json_data(RunEventDTO::from_event(event))
                        .expect("failed to serialize event data"),
                ),
                Err(err) => Ok(
                    Event::default()
                        .event("error")
                        .json_data(ErrorResponseDTO::from(match err {
                            JobQueueError::TaskExecution { source, .. } => {
                                match source.downcast::<ServiceError>() {
                                    Ok(err) => ApiError::from(*err),
                                    Err(source) => ApiError::unexpected_error(source.to_string()),
                                }
                            }
                            err => ApiError::unexpected_error(err.to_string()),
                        }))
                        .expect("failed to serialize error response")
                ),
            }
        }))
        .keep_alive(
            KeepAlive::new()
                .interval(Duration::from_secs(15))
                .text("keep-alive"),
        )
        .into_response(),
        Err(err) => {
            ErrorResponseDTO::from(ApiError::unexpected_error(err.to_string()))
                .into_response()
        }
    }
}

/// Start a new run without waiting for it to finish
#[utoipa::path(
    post,
    path = "/runs/start",
    operation_id = "start_run",
    tag = "runs",
    description = "Start a new run and return immediately with its run ID.",
    params(
        ("X-Tenant_id" = Option<TenantIdHeader>, Header, description = "The ID of the tenant"),
    ),
    request_body = StartRunRequestDTO,
    responses(
        (status = 202, description = "The run was started", body = StartRunResponseDTO),
        (status = 400, description = "Bad request", body = ErrorResponseDTO),
        (status = 404, description = "Session not found", body = ErrorResponseDTO),
        (status = 500, description = "Internal server error", body = ErrorResponseDTO)
    ),
)]
async fn start_run_endpoint(
    State(state): State<ReActApiState>,
    tenant_id: Option<TenantIdHeader>,
    Json(payload): Json<StartRunRequestDTO>,
) -> Response
{
    if let Err(err) = payload.validate() {
        return ErrorResponseDTO::from(ApiError::from(err))
            .into_response();
    }

    match state
        .task_queue
        .enqueue_stream(JobStreamOptions::new(RunStreamTask::new(
            (*state.service).clone(),
            payload.to_params(
                tenant_id.map_or(
                    state.default_tenant_id.into_inner(),
                    TenantIdHeader::into_inner,
                ),
            ),
            CancellationToken::new(),
            state.bus.clone(),
        )))
        .await
    {
        Ok(mut stream) => {
            let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();

            tokio::spawn(async move {
                if stream.next().await.is_some() {
                    let _ = ready_tx.send(());
                }

                while stream.next().await.is_some() {}
            });

            let _ = tokio::time::timeout(Duration::from_secs(5), ready_rx).await;

            (
                StatusCode::ACCEPTED,
                Json(StartRunResponseDTO {
                    run_id: payload.run_id,
                    session_id: payload.session_id,
                }),
            )
                .into_response()
        }
        Err(err) => ErrorResponseDTO::from(ApiError::unexpected_error(err.to_string()))
            .into_response(),
    }
}

/// Cancel a running run
#[utoipa::path(
    put,
    path = "/runs/{run_id}/cancel",
    operation_id = "cancel_run",
    tag = "runs",
    description = "Cancel a run that is currently active.",
    params(
        ("run_id" = Uuid, Path, description = "The ID of the run"),
        ("X-Tenant_id" = Option<TenantIdHeader>, Header, description = "The ID of the tenant"),
    ),
    responses(
        (status = 204, description = "The cancellation request was processed"),
        (status = 404, description = "Run not found", body = ErrorResponseDTO),
        (status = 500, description = "Internal server error", body = ErrorResponseDTO)
    ),
)]
async fn cancel_run_endpoint(
    State(state): State<ReActApiState>,
    Path(run_id): Path<Uuid>,
    tenant_id: Option<TenantIdHeader>,
) -> Response
{
    let tenant_id = tenant_id.map_or(
        state.default_tenant_id.clone().into_inner(),
        TenantIdHeader::into_inner,
    );

    if let Err(err) = state
        .service
        .get_run(&tenant_id, run_id)
        .await
    {
        return ErrorResponseDTO::from(ApiError::from(err))
            .into_response();
    }

    match state
        .service
        .cancel_run(&tenant_id, run_id)
        .await
    {
        Ok(_) => StatusCode::NO_CONTENT
            .into_response(),
        Err(err) => ErrorResponseDTO::from(ApiError::from(err))
            .into_response(),
    }
}

/// Reattach to a currently active run and stream its events from now on
#[utoipa::path(
    get,
    path = "/runs/{run_id}/events",
    operation_id = "reattach_run",
    tag = "runs",
    description = "Open a streaming connection to a run that is currently executing andreceive its events from this point forward.",
    params(
        ("run_id" = Uuid, Path, description = "The ID of the run"),
        ("X-Tenant_id" = Option<TenantIdHeader>, Header, description = "The ID of the tenant"),
    ),
    responses(
        (status = 200, description = "A stream of events", content_type = "text/event-stream"),
        (status = 404, description = "Run not found", body = ErrorResponseDTO),
        (status = 409, description = "Run is not currently active", body = ErrorResponseDTO),
        (status = 500, description = "Internal server error", body = ErrorResponseDTO)
    ),
)]
async fn reattach_run_endpoint(
    State(state): State<ReActApiState>,
    Path(run_id): Path<Uuid>,
    tenant_id: Option<TenantIdHeader>,
) -> Response
{
    let tenant_id = tenant_id.map_or(
        state.default_tenant_id.clone().into_inner(),
        TenantIdHeader::into_inner,
    );

    match state
        .service
        .get_run(&tenant_id, run_id)
        .await
    {
        Ok(run) if run.status != RunStatus::Running => {
            return ErrorResponseDTO::from(ApiError::new(
                409013,
                format!("run {run_id} is not currently active"),
            ))
            .into_response();
        }
        Err(err) => return ErrorResponseDTO::from(ApiError::from(err))
            .into_response(),
        Ok(_) => {}
    }

    match state
        .bus
        .topic::<RunEvent>(&format!("run:{tenant_id}:{run_id}"))
        .subscribe()
        .await
    {
        Ok(subscription) => Sse::new(subscription.filter_map(|result| async move {
            result.ok().map(|event| {
                Ok::<_, Infallible>(
                    Event::default()
                        .event(event.kind())
                        .json_data(RunEventDTO::from_event(event))
                        .expect("failed to serialize event data"),
                )
            })
        }))
        .keep_alive(
            KeepAlive::new()
                .interval(Duration::from_secs(15))
                .text("keep-alive"),
        )
        .into_response(),
        Err(err) => ErrorResponseDTO::from(ApiError::unexpected_error(err.to_string()))
            .into_response(),
    }
}
