// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use reqwest::{Client, Method, Response};
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware, RequestBuilder};
use reqwest_tracing::TracingMiddleware;

use crate::client::{config::A2aClientConfig, errors::A2aClientError};

#[derive(Debug, Clone)]
pub struct BaseClient {
    client: ClientWithMiddleware,
    config: A2aClientConfig,
}

impl BaseClient {
    pub(crate) fn from_config(config: A2aClientConfig) -> Result<Self, A2aClientError> {
        Ok(Self {
            client: ClientBuilder::new(
                Client::builder()
                    // Set read timeout instead of total timeout to avoid issues with
                    // the stream operation using SSE.
                    .read_timeout(config.timeout)
                    .default_headers(config.default_headers.clone())
                    .build()
                    .map_err(|err| A2aClientError::configuration(err.to_string()))?,
            )
            .with(TracingMiddleware::default())
            .build(),
            config,
        })
    }

    pub(crate) fn config(&self) -> &A2aClientConfig {
        &self.config
    }

    pub(crate) fn request(&self, method: Method, url: &str, query: Option<&str>) -> RequestBuilder {
        self.client
            .request(
                method,
                match query {
                    Some(q) if !q.is_empty() => format!("{}{}?{}", self.config.base_url, url, q),
                    _ => format!("{}{}", self.config.base_url, url),
                },
            )
            .headers(self.config.default_headers.clone())
    }

    pub(crate) async fn send(&self, builder: RequestBuilder) -> Result<Response, A2aClientError> {
        let response = builder.send().await?;

        if response.status().is_success() {
            return Ok(response);
        }

        Err(A2aClientError::response(response.status(), response.text().await?))
    }
}
