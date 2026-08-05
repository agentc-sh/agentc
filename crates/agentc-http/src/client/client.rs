// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::sync::Arc;

use http::Method;
use reqwest::Body;
use reqwest_middleware::ClientWithMiddleware;
use reqwest_tracing::OtelName;

use crate::client::{
    builder::HttpClientBuilder,
    errors::HttpClientError,
    limits::Limits,
    request::{HttpRequest, HttpRequestBuilder},
    response::HttpResponse,
};

/// A policy-controlled HTTP client.
///
/// Cloning is cheap and shares one connection pool.
#[derive(Clone)]
pub struct HttpClient {
    inner: Arc<HttpClientInner>,
}

impl HttpClient {
    /// Creates a builder.
    pub fn builder() -> HttpClientBuilder {
        HttpClientBuilder::new()
    }

    pub(crate) fn from_inner(inner: HttpClientInner) -> Self {
        Self { inner: Arc::new(inner) }
    }

    /// Starts a `GET` request.
    pub fn get(&self, url: impl AsRef<str>) -> HttpRequestBuilder {
        self.request(Method::GET, url)
    }

    /// Starts a `HEAD` request.
    pub fn head(&self, url: impl AsRef<str>) -> HttpRequestBuilder {
        self.request(Method::HEAD, url)
    }

    /// Starts a `POST` request.
    pub fn post(&self, url: impl AsRef<str>) -> HttpRequestBuilder {
        self.request(Method::POST, url)
    }

    /// Starts a `PUT` request.
    pub fn put(&self, url: impl AsRef<str>) -> HttpRequestBuilder {
        self.request(Method::PUT, url)
    }

    /// Starts a `PATCH` request.
    pub fn patch(&self, url: impl AsRef<str>) -> HttpRequestBuilder {
        self.request(Method::PATCH, url)
    }

    /// Starts a `DELETE` request.
    pub fn delete(&self, url: impl AsRef<str>) -> HttpRequestBuilder {
        self.request(Method::DELETE, url)
    }

    /// Starts a request with any method.
    pub fn request(&self, method: Method, url: impl AsRef<str>) -> HttpRequestBuilder {
        HttpRequestBuilder::new(self.inner.clone(), method, url.as_ref())
    }

    /// Sends a prepared request.
    pub async fn execute(
        &self,
        request: impl Into<HttpRequest>,
    ) -> Result<HttpResponse, HttpClientError> {
        self.inner.execute(request.into()).await
    }
}

pub(crate) struct HttpClientInner {
    client: ClientWithMiddleware,
    limits: Limits,
}

impl HttpClientInner {
    pub(crate) fn new(client: ClientWithMiddleware, limits: Limits) -> Self {
        Self { client, limits }
    }

    pub(crate) async fn execute(
        &self,
        request: HttpRequest,
    ) -> Result<HttpResponse, HttpClientError> {
        let parts = request.into_parts();
        let permit = match &self.limits.concurrency {
            Some(semaphore) => Some(
                semaphore
                    .clone()
                    .acquire_owned()
                    .await
                    .map_err(|error| HttpClientError::configuration(error.to_string()))?,
            ),
            None => None,
        };

        let mut builder = self
            .client
            .request(parts.method, parts.url)
            .headers(parts.headers);

        if let Some(body) = parts.body {
            builder = builder.body(Body::from(body));
        }

        if let Some(timeout) = parts
            .timeout
            .or(self.limits.request_timeout)
        {
            builder = builder.timeout(timeout);
        }

        if let Some(label) = parts.label {
            builder = builder.with_extension(OtelName(label));
        }

        Ok(HttpResponse::new(builder.send().await?, self.limits.max_response_bytes, permit))
    }
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use axum::{Router, response::Redirect, routing::get};
    use tokio::net::TcpListener;
    use url::Url;

    use super::*;
    use crate::client::{
        policies::pattern::{PatternPolicy, UrlPattern},
        policy::{Denied, Policy},
    };

    struct DenyAll;

    impl Policy for DenyAll {
        fn name(&self) -> &'static str {
            "deny-all"
        }

        fn check_url(&self, _url: &Url) -> Result<(), Denied> {
            Err(Denied::new("nothing is permitted"))
        }
    }

    async fn server() -> SocketAddr {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("test listener binds");
        let address = listener
            .local_addr()
            .expect("test listener reports its address");

        drop(tokio::spawn(async move {
            axum::serve(
                listener,
                Router::new()
                    .route("/ok", get(|| async { "hello" }))
                    .route("/big", get(|| async { "x".repeat(64) }))
                    .route("/away", get(|| async { Redirect::to("https://denied.test/") })),
            )
            .await
        }));

        address
    }

    #[tokio::test]
    async fn permitted_request_succeeds() {
        let address = server().await;

        assert_eq!(
            HttpClient::builder()
                .build()
                .expect("client builds")
                .get(format!("http://{address}/ok"))
                .send()
                .await
                .expect("request succeeds")
                .text()
                .await
                .expect("body reads"),
            "hello",
        );
    }

    #[tokio::test]
    async fn denied_request_reports_the_policy_name() {
        let address = server().await;

        assert!(matches!(
            HttpClient::builder()
                .policy(DenyAll)
                .build()
                .expect("client builds")
                .get(format!("http://{address}/ok"))
                .send()
                .await,
            Err(HttpClientError::Denied { policy: "deny-all", .. }),
        ));
    }

    #[tokio::test]
    async fn oversized_body_is_rejected() {
        let address = server().await;

        assert!(matches!(
            HttpClient::builder()
                .max_response_bytes(8_u32)
                .build()
                .expect("client builds")
                .get(format!("http://{address}/big"))
                .send()
                .await
                .expect("head arrives")
                .bytes()
                .await,
            Err(HttpClientError::BodyTooLarge { limit: 8 }),
        ));
    }

    #[tokio::test]
    async fn denied_redirect_target_is_refused() {
        let address = server().await;

        assert!(matches!(
            HttpClient::builder()
                .policy(
                    PatternPolicy::allow([UrlPattern::parse(format!("http://{address}/*"))
                        .expect("test pattern parses"),])
                    .expect("test policy builds"),
                )
                .build()
                .expect("client builds")
                .get(format!("http://{address}/away"))
                .send()
                .await,
            Err(HttpClientError::Denied { policy: "url-pattern", .. }),
        ));
    }
}
