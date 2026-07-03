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
use futures::stream::StreamExt;
use std::{convert::Infallible, time::Duration};
use utoipa_axum::{router::OpenApiRouter, routes};

use agentc_http::{
    dto::errors::ErrorResponseDTO,
    errors::ApiError,
    extractors::{Json, TenantIdHeader},
    state::ApiState,
};

use crate::{protocol::input::RunAgentInput, traits::AgUiService};

/// Builds the OpenAPI router for the AG-UI protocol.
pub fn router<S>(state: ApiState<S>) -> OpenApiRouter
where
    S: AgUiService + Clone + Send + Sync + 'static,
    S::Error: Into<ApiError>,
{
    OpenApiRouter::new()
        .routes(routes!(ag_ui_run_endpoint))
        .with_state(state)
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
async fn ag_ui_run_endpoint<S>(
    State(state): State<ApiState<S>>,
    tenant_id: Option<TenantIdHeader>,
    Json(input): Json<RunAgentInput>,
) -> Response
where
    S: AgUiService + Clone + Send + Sync + 'static,
    S::Error: Into<ApiError>,
{
    let tenant_id = tenant_id.map_or(state.default_tenant_id.clone(), TenantIdHeader::into_inner);

    match state.ag_ui_run(input, &tenant_id).await {
        Ok(stream) => Sse::new(stream.map(|result| {
            match result {
                Ok(event) => Ok::<_, Infallible>(
                    Event::default()
                        .event(event.event_type().as_str())
                        .json_data(event)
                        .expect("failed to serialize event data"),
                ),
                Err(err) => Ok(Event::default()
                    .event("error")
                    .json_data(ErrorResponseDTO::from(err.into()))
                    .expect("failed to serialize error response")),
            }
        }))
        .keep_alive(
            KeepAlive::new()
                .interval(Duration::from_secs(15))
                .text("keep-alive"),
        )
        .into_response(),
        Err(err) => ErrorResponseDTO::from(err.into()).into_response(),
    }
}
