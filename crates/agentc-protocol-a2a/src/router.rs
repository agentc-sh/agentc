// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_stream::stream;
use axum::{
    extract::State,
    response::{
        IntoResponse, Response,
        sse::{Event, KeepAlive, Sse},
    },
    routing::post,
};
use futures::{StreamExt, stream::BoxStream};
use jobq::{
    AnyExecutable, Error as JobQueueError, FifoQueue, JobQueue, JobStreamOptions, StreamTask,
};
use std::{convert::Infallible, sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;
use utoipa_axum::{router::OpenApiRouter, routes};

use agentc_http::{
    dto::errors::ErrorResponseDTO,
    errors::ApiError,
    extractors::{Json, Path, TenantIdHeader},
    openapi::OpenApiRouterExt,
    state::DefaultTenantId,
    stream::CancelOnDropStream,
};

use crate::{
    protocol::{
        AgentCard, AgentInterface, CancelTaskRequest, GetTaskRequest, SendMessageRequest,
        SendMessageResponse, StreamResponse, Task, TaskId,
    },
    traits::A2aService,
};

#[derive(Clone)]
struct A2aRouterState {
    service: Arc<dyn A2aService>,
    agent_interface: AgentInterface,
    default_tenant_id: DefaultTenantId,
    task_queue: Arc<JobQueue<FifoQueue<AnyExecutable>>>,
}

struct A2aStreamTask {
    service: Arc<dyn A2aService>,
    request: SendMessageRequest,
    disconnect: CancellationToken,
}

impl A2aStreamTask {
    fn new(
        service: Arc<dyn A2aService>,
        request: SendMessageRequest,
        disconnect: CancellationToken,
    ) -> Self {
        Self { service, request, disconnect }
    }
}

impl StreamTask for A2aStreamTask {
    type Item = StreamResponse;
    type Error = ApiError;

    fn execute(&self) -> BoxStream<'_, Result<Self::Item, Self::Error>> {
        Box::pin(stream! {
            let mut stream = match self.service
                .stream_message(self.request.clone())
                .await
            {
                Ok(stream) => stream,
                Err(err) => {
                    yield Err(err);
                    return;
                }
            };

            loop {
                match tokio::select! {
                    _ = self.disconnect.cancelled() => match stream.cancel().await {
                        Ok(()) => Ok(None),
                        Err(err) => Err(err),
                    },
                    event = stream.next() => {
                        match event {
                            Some(Ok(event)) => Ok(Some(event)),
                            Some(Err(err)) => Err(err),
                            None => Ok(None),
                        }
                    }
                } {
                    Ok(Some(event)) => yield Ok(event),
                    Ok(None) => break,
                    Err(err) => {
                        yield Err(err);
                        break;
                    }
                }
            }
        })
    }
}

/// Builds the OpenAPI router for the A2A protocol.
pub fn router(
    service: Arc<dyn A2aService>,
    agent_interface: AgentInterface,
    default_tenant_id: DefaultTenantId,
    task_queue: Arc<JobQueue<FifoQueue<AnyExecutable>>>,
) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(agent_card_endpoint))
        .routes(routes!(send_message_endpoint))
        .routes(routes!(stream_message_endpoint))
        .routes(routes!(get_task_endpoint))
        .route("/tasks/{id}", post(post_task_action_endpoint))
        .document::<__path_cancel_task_endpoint>()
        .with_state(A2aRouterState {
            service,
            agent_interface,
            default_tenant_id,
            task_queue,
        })
}

/// Returns the agent card for the A2A service.
#[utoipa::path(
    get,
    path = "/.well-known/agent-card.json",
    operation_id = "a2a_agent_card",
    tag = "a2a",
    description = "Returns the agent card describing the A2A service.",
    responses(
        (status = 200, description = "The agent card", body = AgentCard),
        (status = 500, description = "Internal server error", body = ErrorResponseDTO)
    )
)]
async fn agent_card_endpoint(State(state): State<A2aRouterState>) -> Response {
    Json(
        state
            .service
            .agent_card(&state.agent_interface),
    )
    .into_response()
}

/// Sends a message to the A2A service and waits for its task response.
#[utoipa::path(
    post,
    path = "/message:send",
    operation_id = "a2a_send_message",
    tag = "a2a",
    description = "Sends a message to the agent and returns the resulting task or message.",
    params(
        ("X-Tenant_id" = Option<TenantIdHeader>, Header, description = "The ID of the tenant"),
    ),
    request_body = SendMessageRequest,
    responses(
        (status = 200, description = "The task or message response", body = SendMessageResponse),
        (status = 400, description = "Bad request", body = ErrorResponseDTO),
        (status = 500, description = "Internal server error", body = ErrorResponseDTO)
    )
)]
async fn send_message_endpoint(
    State(state): State<A2aRouterState>,
    tenant_id: Option<TenantIdHeader>,
    Json(mut request): Json<SendMessageRequest>,
) -> Response {
    request.tenant = request
        .tenant
        .or_else(|| tenant_id.map(TenantIdHeader::into_inner))
        .or_else(|| {
            Some(
                state
                    .default_tenant_id
                    .clone()
                    .into_inner(),
            )
        });

    match state
        .service
        .send_message(request)
        .await
    {
        Ok(response) => Json(response).into_response(),
        Err(err) => ErrorResponseDTO::from(err).into_response(),
    }
}

/// Sends a message to the A2A service and streams task updates.
#[utoipa::path(
    post,
    path = "/message:stream",
    operation_id = "a2a_stream_message",
    tag = "a2a",
    description = "Sends a message to the agent and streams task updates using SSE.",
    params(
        ("X-Tenant_id" = Option<TenantIdHeader>, Header, description = "The ID of the tenant"),
    ),
    request_body = SendMessageRequest,
    responses(
        (status = 200, description = "A stream of task updates", content_type = "text/event-stream"),
        (status = 400, description = "Bad request", body = ErrorResponseDTO),
        (status = 500, description = "Internal server error", body = ErrorResponseDTO)
    )
)]
async fn stream_message_endpoint(
    State(state): State<A2aRouterState>,
    tenant_id: Option<TenantIdHeader>,
    Json(mut request): Json<SendMessageRequest>,
) -> Response {
    request.tenant = request
        .tenant
        .or_else(|| tenant_id.map(TenantIdHeader::into_inner))
        .or_else(|| {
            Some(
                state
                    .default_tenant_id
                    .clone()
                    .into_inner(),
            )
        });

    let disconnect = CancellationToken::new();

    match state
        .task_queue
        .enqueue_stream(JobStreamOptions::new(A2aStreamTask::new(
            state.service.clone(),
            request,
            disconnect.clone(),
        )))
        .await
    {
        Ok(handle) => Sse::new(CancelOnDropStream::new(handle, disconnect).map(|result| {
            match result {
                Ok(event) => Ok::<_, Infallible>(
                    Event::default()
                        .json_data(event)
                        .expect("failed to serialize event data"),
                ),
                Err(err) => Ok(Event::default()
                    .event("error")
                    .json_data(ErrorResponseDTO::from(match err {
                        JobQueueError::TaskExecution { source, .. } => {
                            match source.downcast::<ApiError>() {
                                Ok(err) => *err,
                                Err(source) => ApiError::unexpected_error(source.to_string()),
                            }
                        }
                        err => ApiError::unexpected_error(err.to_string()),
                    }))
                    .expect("failed to serialize error response")),
            }
        }))
        .keep_alive(
            KeepAlive::new()
                .interval(Duration::from_secs(15))
                .text("keep-alive"),
        )
        .into_response(),
        Err(err) => {
            ErrorResponseDTO::from(ApiError::unexpected_error(err.to_string())).into_response()
        }
    }
}

/// Retrieves a task by ID.
#[utoipa::path(
    get,
    path = "/tasks/{id}",
    operation_id = "a2a_get_task",
    tag = "a2a",
    description = "Retrieves the current state of an A2A task.",
    params(
        ("id" = String, Path, description = "The task ID"),
        ("X-Tenant_id" = Option<TenantIdHeader>, Header, description = "The ID of the tenant"),
    ),
    responses(
        (status = 200, description = "The task", body = Task),
        (status = 400, description = "Bad request", body = ErrorResponseDTO),
        (status = 404, description = "Task not found", body = ErrorResponseDTO),
        (status = 500, description = "Internal server error", body = ErrorResponseDTO)
    )
)]
async fn get_task_endpoint(
    State(state): State<A2aRouterState>,
    tenant_id: Option<TenantIdHeader>,
    Path(id): Path<TaskId>,
) -> Response {
    match state
        .service
        .get_task(GetTaskRequest {
            id,
            history_length: None,
            tenant: tenant_id
                .map(TenantIdHeader::into_inner)
                .or_else(|| {
                    Some(
                        state
                            .default_tenant_id
                            .clone()
                            .into_inner(),
                    )
                }),
        })
        .await
    {
        Ok(task) => Json(task).into_response(),
        Err(err) => ErrorResponseDTO::from(err).into_response(),
    }
}

async fn post_task_action_endpoint(
    State(state): State<A2aRouterState>,
    tenant_id: Option<TenantIdHeader>,
    Path(id_and_action): Path<String>,
) -> Response {
    let Some((id, action)) = id_and_action.rsplit_once(':') else {
        return ErrorResponseDTO::from(ApiError::not_found(format!(
            "unknown task action: {id_and_action}"
        )))
        .into_response();
    };

    match action {
        "cancel" => cancel_task_endpoint(State(state), tenant_id, Path(TaskId::from(id))).await,
        _ => ErrorResponseDTO::from(ApiError::not_found(format!("unknown task action: {action}")))
            .into_response(),
    }
}

/// Cancels a task by ID.
#[utoipa::path(
    post,
    path = "/tasks/{id}:cancel",
    operation_id = "a2a_cancel_task",
    tag = "a2a",
    description = "Cancels an active A2A task.",
    params(
        ("id" = String, Path, description = "The task ID"),
        ("X-Tenant_id" = Option<TenantIdHeader>, Header, description = "The ID of the tenant"),
    ),
    responses(
        (status = 200, description = "The canceled task", body = Task),
        (status = 400, description = "Bad request", body = ErrorResponseDTO),
        (status = 404, description = "Task not found", body = ErrorResponseDTO),
        (status = 500, description = "Internal server error", body = ErrorResponseDTO)
    )
)]
async fn cancel_task_endpoint(
    State(state): State<A2aRouterState>,
    tenant_id: Option<TenantIdHeader>,
    Path(id): Path<TaskId>,
) -> Response {
    match state
        .service
        .cancel_task(CancelTaskRequest {
            id,
            metadata: None,
            tenant: tenant_id
                .map(TenantIdHeader::into_inner)
                .or_else(|| {
                    Some(
                        state
                            .default_tenant_id
                            .clone()
                            .into_inner(),
                    )
                }),
        })
        .await
    {
        Ok(task) => Json(task).into_response(),
        Err(err) => ErrorResponseDTO::from(err).into_response(),
    }
}
