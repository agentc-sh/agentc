// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use reqwest::{Client, Method, Response, header::HeaderMap};
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware, RequestBuilder};
use reqwest_tracing::TracingMiddleware;

use crate::client::{config::A2aClientConfig, errors::A2aClientError};

#[derive(Debug, Clone)]
pub struct BaseClient {
    client: ClientWithMiddleware,
    base_url: String,
    default_headers: HeaderMap,
}

impl BaseClient {
    pub(crate) fn from_config(config: A2aClientConfig) -> Result<Self, A2aClientError> {
        Ok(Self {
            base_url: config.base_url,
            client: ClientBuilder::new(
                Client::builder()
                    .timeout(config.timeout)
                    .default_headers(config.default_headers.clone())
                    .build()
                    .map_err(|err| A2aClientError::configuration(err.to_string()))?,
            )
            .with(TracingMiddleware::default())
            .build(),
            default_headers: config.default_headers,
        })
    }

    pub(crate) fn request(&self, method: Method, url: &str, query: Option<&str>) -> RequestBuilder {
        self.client
            .request(
                method,
                match query {
                    Some(q) if !q.is_empty() => format!("{}{}?{}", self.base_url, url, q),
                    _ => format!("{}{}", self.base_url, url),
                },
            )
            .headers(self.default_headers.clone())
    }

    pub(crate) async fn send(&self, builder: RequestBuilder) -> Result<Response, A2aClientError> {
        let response = builder.send().await?;

        if response.status().is_success() {
            return Ok(response);
        }

        Err(A2aClientError::response(response.status(), response.text().await?))
    }
}
