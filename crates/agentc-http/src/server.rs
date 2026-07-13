// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use anyhow::{Result, anyhow};
use axum::{
    Router,
    body::Body,
    extract::DefaultBodyLimit,
    response::{IntoResponse, Json},
    routing::get,
};
use axum_server::Handle;
use http::{Request, Response};
use sentry::integrations::tower::{NewSentryLayer, SentryHttpLayer};
use serde::Serialize;
use std::{net::SocketAddr, time::Duration};
use tokio::task::JoinHandle;
use tower::ServiceBuilder;
use tower_http::trace::{DefaultMakeSpan, TraceLayer};
use utoipa::{ToSchema, openapi::OpenApi};
use utoipa_axum::{router::OpenApiRouter, routes};
use utoipa_scalar::{Scalar, Servable as ScalarServable};

#[cfg(feature = "tls")]
use axum_server::tls_rustls::RustlsConfig;

use agentc_telemetry::{Span, info};

use crate::{dto::errors::ErrorResponseDTO, errors::ApiError};

#[derive(Serialize, ToSchema)]
struct PingResponse {
    version: &'static str,
}

#[utoipa::path(
    get,
    path = "/",
    summary = "Ping endpoint",
    description = "The root endpoint that can be used to check if the server is running and to get the current version of the API.",
    operation_id = "ping",
    tag = "ping",
    responses(
        (status = 200, description = "Ping endpoint", body = PingResponse),
    ),
)]
async fn ping_endpoint() -> impl IntoResponse {
    Json(PingResponse { version: env!("CARGO_PKG_VERSION") })
}

async fn not_found_fallback() -> impl IntoResponse {
    ErrorResponseDTO::from(ApiError::not_found("not found")).into_response()
}

pub fn merge_routers<S>(routers: impl IntoIterator<Item = OpenApiRouter<S>>) -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
{
    routers
        .into_iter()
        .fold(OpenApiRouter::new(), |acc, router| acc.merge(router))
}

#[cfg(feature = "tls")]
pub async fn tls_config(cert_file: String, key_file: String) -> RustlsConfig {
    RustlsConfig::from_pem_file(cert_file, key_file)
        .await
        .expect("Failed to create TLS config")
}

pub struct HttpServer {
    // Stored as Option so spawn()/spawn_tls() can move it into the background task.
    router: Option<Router>,
    addr: SocketAddr,
    handle: Handle<SocketAddr>,
    task: Option<JoinHandle<Result<()>>>,
}

impl HttpServer {
    pub fn builder() -> HttpServerBuilder {
        HttpServerBuilder::new()
    }

    /// Get the local address that the server is bound to.
    pub fn address(&self) -> &SocketAddr {
        &self.addr
    }

    /// Spawn the HTTP server as a background task. Must be called before
    /// [`graceful_shutdown`](HttpServer::graceful_shutdown) or [`join`](HttpServer::join).
    pub fn spawn(&mut self) {
        let router = self
            .router
            .take()
            .expect("server already spawned");
        let addr = self.addr;
        let handle = self.handle.clone();

        self.task = Some(tokio::spawn(async move {
            axum_server::bind(addr)
                .handle(handle)
                .serve(router.into_make_service_with_connect_info::<SocketAddr>())
                .await?;
            Ok(())
        }));
    }

    /// Spawn the TLS server as a background task.
    #[cfg(feature = "tls")]
    pub fn spawn_tls(&mut self, tls_config: RustlsConfig) {
        let router = self
            .router
            .take()
            .expect("server already spawned");
        let addr = self.addr;
        let handle = self.handle.clone();
        self.task = Some(tokio::spawn(async move {
            axum_server::bind_rustls(addr, tls_config)
                .handle(handle)
                .serve(router.into_make_service_with_connect_info::<SocketAddr>())
                .await?;
            Ok(())
        }));
    }

    /// Signal the server to stop accepting new connections and drain in-flight
    /// requests within the given timeout.
    pub fn graceful_shutdown(&self, timeout: Option<Duration>) {
        self.handle.graceful_shutdown(timeout);
    }

    /// Wait for the server task to finish. Should be called after
    /// [`graceful_shutdown`](HttpServer::graceful_shutdown).
    pub async fn join(mut self) -> Result<()> {
        match self.task.take() {
            Some(task) => task
                .await
                .map_err(|e| anyhow!("server task panicked: {e}"))?,
            None => Ok(()),
        }
    }
}

pub struct HttpServerBuilder {
    routers: Vec<OpenApiRouter>,
    openapi: Option<OpenApi>,
    host: String,
    port: u16,
    max_request_size: Option<usize>,
}

impl Default for HttpServerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpServerBuilder {
    pub fn new() -> Self {
        Self {
            routers: Vec::new(),
            openapi: None,
            host: "127.0.0.1".to_string(),
            port: 8080,
            max_request_size: None,
        }
    }

    pub fn with_host(mut self, host: impl Into<String>) -> Self {
        self.host = host.into();
        self
    }

    pub fn with_port(mut self, port: impl Into<u16>) -> Self {
        self.port = port.into();
        self
    }

    pub fn with_openapi(mut self, openapi: OpenApi) -> Self {
        self.openapi = Some(openapi);
        self
    }

    pub fn with_router(mut self, router: OpenApiRouter) -> Self {
        self.routers.push(router);
        self
    }

    pub fn with_max_request_size(mut self, max_request_size: usize) -> Self {
        self.max_request_size = Some(max_request_size);
        self
    }

    pub fn build(self) -> Result<HttpServer> {
        let addr = format!("{}:{}", self.host, self.port).parse::<SocketAddr>()?;
        let handle = Handle::<SocketAddr>::new();
        let openapi = self
            .openapi
            .expect("OpenAPI document must be provided");

        let (mut router, api_doc) = OpenApiRouter::with_openapi(openapi.clone())
            .merge(merge_routers(self.routers))
            .layer(
                ServiceBuilder::new()
                    .layer(NewSentryLayer::new_from_top())
                    .layer(SentryHttpLayer::new())
                    .layer(
                        TraceLayer::new_for_http()
                            .make_span_with(DefaultMakeSpan::new().include_headers(false))
                            .on_request(|request: &Request<Body>, span: &Span| {
                                span.record("method", request.method().as_str())
                                    .record("uri", request.uri().to_string());

                                info!(
                                    parent: span,
                                    event = "ReceivedRequest",
                                    method = %request.method(),
                                    uri = %request.uri(),
                                );
                            })
                            .on_response(
                                |response: &Response<Body>, latency: Duration, span: &Span| {
                                    info!(
                                        parent: span,
                                        event = "RequestFinished",
                                        status = %response.status(),
                                        latency = ?latency,
                                    );
                                },
                            ),
                    ),
            )
            // Ping endpoint after the tracing layer to ensure
            // that it is not traced
            .routes(routes!(ping_endpoint))
            .fallback(not_found_fallback)
            .split_for_parts();

        router = router
            .merge(Scalar::with_url("/.well-known/docs", api_doc.clone()))
            .route("/.well-known/openapi.json", get(move || async move { Json(api_doc) }));

        if let Some(max_request_size) = self.max_request_size {
            router = router.layer(DefaultBodyLimit::max(max_request_size));
        }

        Ok(HttpServer {
            router: Some(router),
            addr,
            handle,
            task: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use axum::{body::Bytes, routing::post};
    use http::StatusCode;
    use tower::ServiceExt;
    use utoipa::openapi::OpenApiBuilder;

    async fn echo_endpoint(_body: Bytes) -> impl IntoResponse {
        StatusCode::OK
    }

    fn bounded_router() -> Router {
        let mut server = HttpServer::builder()
            .with_openapi(OpenApiBuilder::new().build())
            .with_router(OpenApiRouter::new().route("/test", post(echo_endpoint)))
            .with_max_request_size(8)
            .build()
            .expect("server builds");

        server
            .router
            .take()
            .expect("router present")
    }

    #[tokio::test]
    async fn under_limit_request_succeeds() {
        let response = bounded_router()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/test")
                    .body(Body::from("abc"))
                    .expect("request builds"),
            )
            .await
            .expect("service responds");

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn over_limit_request_is_rejected() {
        let response = bounded_router()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/test")
                    .body(Body::from("x".repeat(64)))
                    .expect("request builds"),
            )
            .await
            .expect("service responds");

        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }
}
