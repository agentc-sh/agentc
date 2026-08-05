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
};
use futures::{StreamExt, stream::BoxStream};
use jobq::{
    AnyExecutable, Error as JobQueueError, FifoQueue, JobQueue, JobStreamOptions, StreamTask,
};
use std::{convert::Infallible, sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;
use utoipa_axum::{router::OpenApiRouter, routes};

use agentc_http::server::{
    dto::errors::ErrorResponseDTO,
    errors::ApiError,
    extractors::{Json, TenantIdHeader},
    state::DefaultTenantId,
    stream::CancelOnDropStream,
};

use crate::{
    protocol::{event::Event as AgUiEvent, input::RunAgentInput},
    traits::AgUiService,
};

#[derive(Clone)]
struct AgUiRouterState {
    service: Arc<dyn AgUiService>,
    default_tenant_id: DefaultTenantId,
    task_queue: Arc<JobQueue<FifoQueue<AnyExecutable>>>,
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

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use futures::{StreamExt, stream};
    use jobq::BatchJobQueueSystemBuilder;
    use serde_json::Value;
    use std::sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    };

    use crate::{
        protocol::{
            event::{BaseEvent, Event as AgUiEvent, RunStartedEvent},
            ids::{RunId, ThreadId},
            input::RunAgentInput,
        },
        traits::{AgUiRunCancel, AgUiRunStream},
    };

    use super::*;

    struct TestService {
        cancelled: Arc<AtomicBool>,
    }

    #[async_trait]
    impl AgUiService for TestService {
        async fn ag_ui_run(
            &self,
            _input: RunAgentInput,
            _tenant_id: &str,
        ) -> Result<AgUiRunStream, ApiError> {
            Ok(AgUiRunStream::new(stream::pending::<Result<AgUiEvent, ApiError>>().boxed())
                .with_cancel(TestCancel { cancelled: self.cancelled.clone() }))
        }
    }

    struct TestCancel {
        cancelled: Arc<AtomicBool>,
    }

    #[async_trait]
    impl AgUiRunCancel for TestCancel {
        async fn cancel(&self) -> Result<(), ApiError> {
            self.cancelled
                .store(true, Ordering::SeqCst);

            Ok(())
        }
    }

    struct StreamingTestService {
        cancelled: Arc<AtomicBool>,
        emitted: Arc<AtomicUsize>,
        total_events: usize,
    }

    #[async_trait]
    impl AgUiService for StreamingTestService {
        async fn ag_ui_run(
            &self,
            _input: RunAgentInput,
            _tenant_id: &str,
        ) -> Result<AgUiRunStream, ApiError> {
            let emitted = self.emitted.clone();
            let total_events = self.total_events;

            Ok(AgUiRunStream::new(Box::pin(async_stream::stream! {
                for _ in 0..total_events {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                    emitted.fetch_add(1, Ordering::SeqCst);

                    yield Ok(AgUiEvent::RunStarted(RunStartedEvent {
                        base: BaseEvent {
                            timestamp: None,
                            raw_event: None,
                        },
                        thread_id: ThreadId::random(),
                        run_id: RunId::random(),
                    }));
                }
            }))
            .with_cancel(TestCancel { cancelled: self.cancelled.clone() }))
        }
    }

    #[tokio::test]
    async fn ag_ui_stream_task_cancels_run_stream_on_disconnect() {
        let cancelled = Arc::new(AtomicBool::new(false));
        let disconnect = CancellationToken::new();
        let task = AgUiStreamTask::new(
            Arc::new(TestService { cancelled: cancelled.clone() }),
            RunAgentInput::new(
                ThreadId::random(),
                RunId::random(),
                Value::Null,
                vec![],
                vec![],
                vec![],
                Value::Null,
            ),
            "tenant",
            disconnect.clone(),
        );
        let mut stream = task.execute();

        disconnect.cancel();

        assert!(stream.next().await.is_none());
        assert!(cancelled.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn ag_ui_run_endpoint_cancels_stream_task_when_response_is_dropped() {
        let cancelled = Arc::new(AtomicBool::new(false));
        let (task_queue, worker_pool) =
            BatchJobQueueSystemBuilder::<FifoQueue<AnyExecutable>>::fifo(16)
                .with_num_workers(1)
                .build();
        let worker_pool_handle = {
            let worker_pool = worker_pool.clone();

            tokio::spawn(async move {
                worker_pool.run().await;
            })
        };

        drop(
            ag_ui_run_endpoint(
                State(AgUiRouterState {
                    service: Arc::new(TestService { cancelled: cancelled.clone() }),
                    default_tenant_id: DefaultTenantId::new("tenant"),
                    task_queue,
                }),
                None,
                Json(RunAgentInput::new(
                    ThreadId::random(),
                    RunId::random(),
                    Value::Null,
                    vec![],
                    vec![],
                    vec![],
                    Value::Null,
                )),
            )
            .await,
        );

        tokio::time::timeout(Duration::from_secs(1), async {
            while !cancelled.load(Ordering::SeqCst) {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();

        worker_pool.shutdown().await;
        worker_pool_handle.await.unwrap();
    }

    #[tokio::test]
    async fn ag_ui_run_endpoint_stops_streaming_after_response_body_is_dropped() {
        const TOTAL_EVENTS: usize = 32;

        let cancelled = Arc::new(AtomicBool::new(false));
        let emitted = Arc::new(AtomicUsize::new(0));
        let (task_queue, worker_pool) =
            BatchJobQueueSystemBuilder::<FifoQueue<AnyExecutable>>::fifo(16)
                .with_num_workers(1)
                .build();
        let worker_pool_handle = {
            let worker_pool = worker_pool.clone();

            tokio::spawn(async move {
                worker_pool.run().await;
            })
        };
        let mut body = ag_ui_run_endpoint(
            State(AgUiRouterState {
                service: Arc::new(StreamingTestService {
                    cancelled: cancelled.clone(),
                    emitted: emitted.clone(),
                    total_events: TOTAL_EVENTS,
                }),
                default_tenant_id: DefaultTenantId::new("tenant"),
                task_queue,
            }),
            None,
            Json(RunAgentInput::new(
                ThreadId::random(),
                RunId::random(),
                Value::Null,
                vec![],
                vec![],
                vec![],
                Value::Null,
            )),
        )
        .await
        .into_body()
        .into_data_stream();

        tokio::time::timeout(Duration::from_secs(1), body.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();

        drop(body);

        tokio::time::timeout(Duration::from_secs(1), async {
            while !cancelled.load(Ordering::SeqCst) {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();

        worker_pool.shutdown().await;
        worker_pool_handle.await.unwrap();

        assert!(cancelled.load(Ordering::SeqCst));
        assert!(emitted.load(Ordering::SeqCst) > 0);
        assert!(emitted.load(Ordering::SeqCst) < TOTAL_EVENTS);
    }
}

/// Builds the OpenAPI router for the AG-UI protocol.
pub fn router(
    service: Arc<dyn AgUiService>,
    default_tenant_id: DefaultTenantId,
    task_queue: Arc<JobQueue<FifoQueue<AnyExecutable>>>,
) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(ag_ui_run_endpoint))
        .with_state(AgUiRouterState { service, default_tenant_id, task_queue })
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
    let tenant_id =
        tenant_id.map_or(state.default_tenant_id.into_inner(), TenantIdHeader::into_inner);
    let disconnect = CancellationToken::new();

    match state
        .task_queue
        .enqueue_stream(JobStreamOptions::new(AgUiStreamTask::new(
            state.service.clone(),
            input,
            tenant_id,
            disconnect.clone(),
        )))
        .await
    {
        Ok(handle) => Sse::new(CancelOnDropStream::new(handle, disconnect).map(|result| {
            match result {
                Ok(event) => Ok::<_, Infallible>(
                    Event::default()
                        .event(event.event_type().as_str())
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
