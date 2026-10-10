// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::sync::Arc;

use async_trait::async_trait;
use http::Extensions;
use reqwest::{Request, Response};
use reqwest_middleware::{Error as MiddlewareError, Middleware, Next, Result as MiddlewareResult};

use crate::client::{
    errors::HttpClientError,
    policy::{Policy, RequestContext, ResponseContext},
};

pub(crate) struct PolicyMiddleware {
    policies: Arc<[Arc<dyn Policy>]>,
}

impl PolicyMiddleware {
    pub(crate) fn new(policies: Arc<[Arc<dyn Policy>]>) -> Self {
        Self { policies }
    }
}

#[async_trait]
impl Middleware for PolicyMiddleware {
    async fn handle(
        &self,
        request: Request,
        extensions: &mut Extensions,
        next: Next<'_>,
    ) -> MiddlewareResult<Response> {
        for policy in self.policies.iter() {
            policy
                .check_request(&RequestContext::new(
                    request.method(),
                    request.url(),
                    request.headers(),
                ))
                .map_err(|denial| {
                    MiddlewareError::middleware(HttpClientError::denied(policy.name(), denial))
                })?;
        }

        let response = next.run(request, extensions).await?;

        for policy in self.policies.iter() {
            policy
                .check_response(&ResponseContext::new(
                    response.status(),
                    response.url(),
                    response.headers(),
                    response.content_length(),
                ))
                .map_err(|denial| {
                    MiddlewareError::middleware(HttpClientError::denied(policy.name(), denial))
                })?;
        }

        Ok(response)
    }
}
