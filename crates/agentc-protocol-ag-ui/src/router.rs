// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use axum::{
    extract::State,
    response::{
        IntoResponse, Response,
        sse::{Event, KeepAlive, Sse},
    },
};
use async_stream::stream;
use futures::{
    StreamExt,
    stream::BoxStream,
};
use jobq::{
    Error as JobQueueError,
    Executable,
    FifoQueue,
    JobQueue,
    JobStreamOptions,
    StreamTask,
};
use std::{
    convert::Infallible,
    sync::Arc,
    time::Duration,
};
use tokio_util::sync::CancellationToken;
use utoipa_axum::{router::OpenApiRouter, routes};

use agentc_http::{
    dto::errors::ErrorResponseDTO,
    errors::ApiError,
    extractors::{Json, TenantIdHeader},
    state::DefaultTenantId,
    stream::CancelOnDropStream,
};

use crate::{
    protocol::{
        event::Event as AgUiEvent,
        input::RunAgentInput,
    },
    traits::AgUiService,
};

#[derive(Clone)]
struct AgUiRouterState {
    service: Arc<dyn AgUiService>,
    default_tenant_id: DefaultTenantId,
    task_queue: Arc<JobQueue<FifoQueue<Box<dyn Executable>>>>,
}

struct AgUiStreamTask {
    service: Arc<dyn AgUiService>,
    input: RunAgentInput,
    tenant_id: String,
    disconnect: CancellationToken,
}

impl AgUiStreamTask {
    fn new(
        service: Arc<dyn AgUiService>,
        input: RunAgentInput,
        tenant_id: impl Into<String>,
        disconnect: CancellationToken,
    ) -> Self {
        Self {
            service,
            input,
            tenant_id: tenant_id.into(),
            disconnect,
        }
    }
}

impl StreamTask for AgUiStreamTask {
    type Item = AgUiEvent;
    type Error = ApiError;

    fn execute(&self) -> BoxStream<'_, Result<Self::Item, Self::Error>> {
        Box::pin(stream! {
            let mut stream = match self.service
                .ag_ui_run(self.input.clone(), &self.tenant_id)
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

/// Builds the OpenAPI router for the AG-UI protocol.
pub fn router(
    service: Arc<dyn AgUiService>,
    default_tenant_id: DefaultTenantId,
    task_queue: Arc<JobQueue<FifoQueue<Box<dyn Executable>>>>,
) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(ag_ui_run_endpoint))
        .with_state(AgUiRouterState {
            service,
            default_tenant_id,
            task_queue,
        })
}

/// AG-UI Run endpoint
#[utoipa::path(
    post,
    path = "/run",
    operation_id = "ag_ui_run",
    tag = "ag_ui",
    description = "Run an agent with the given input and receive a stream of events in response following the AG-UI protocol.",
    params(
        ("X-Tenant_id" = Option<TenantIdHeader>, Header, description = "The ID of the tenant"),
    ),
    request_body = RunAgentInput,
    responses(
        (status = 200, description = "A stream of events", content_type = "text/event-stream"),
        (status = 400, description = "Bad request", body = ErrorResponseDTO),
        (status = 500, description = "Internal server error", body = ErrorResponseDTO)
    )
)]
async fn ag_ui_run_endpoint(
    State(state): State<AgUiRouterState>,
    tenant_id: Option<TenantIdHeader>,
    Json(input): Json<RunAgentInput>,
) -> Response {
    let tenant_id = tenant_id.map_or(state.default_tenant_id.into_inner(), TenantIdHeader::into_inner);
    let disconnect = CancellationToken::new();

    match state
        .task_queue
        .enqueue_stream(JobStreamOptions::new(AgUiStreamTask::new(
            state.service,
            input,
            tenant_id,
            disconnect.clone(),
        )))
        .await
    {
        Ok(stream) => Sse::new(CancelOnDropStream::new(stream, disconnect).map(|result| {
            match result {
                Ok(event) => Ok::<_, Infallible>(
                    Event::default()
                        .event(event.event_type().as_str())
                        .json_data(event)
                        .expect("failed to serialize event data"),
                ),
                Err(err) => Ok(
                    Event::default()
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
