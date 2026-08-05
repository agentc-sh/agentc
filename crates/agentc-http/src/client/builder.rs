// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::{sync::Arc, time::Duration};

use http::{HeaderMap, HeaderName, HeaderValue};
use reqwest::{Client, redirect};
use reqwest_middleware::{ClientBuilder, Middleware};
use reqwest_tracing::TracingMiddleware;
use tokio::sync::Semaphore;

use crate::client::{
    client::{HttpClient, HttpClientInner},
    errors::{HttpClientError, RedirectRejection},
    limits::Limits,
    middleware::PolicyMiddleware,
    policy::{AddressFilter, Policy, RedirectContext},
    resolver::GuardedResolver,
};

/// Configures an [`HttpClient`](crate::client::client::HttpClient).
///
/// The builder is cloneable and [`build`](crate::client::builder::HttpClientBuilder::build)
/// borrows, so one configuration may produce many clients.
#[derive(Clone)]
pub struct HttpClientBuilder {
    connect_timeout: Option<Duration>,
    read_timeout: Option<Duration>,
    request_timeout: Option<Duration>,
    default_headers: HeaderMap,
    user_agent: Option<String>,
    max_redirects: usize,
    max_response_bytes: Option<u64>,
    concurrency: Option<Arc<Semaphore>>,
    policies: Vec<Arc<dyn Policy>>,
    address_filter: Option<Arc<dyn AddressFilter>>,
    middleware: Vec<Arc<dyn Middleware>>,
    header_error: Option<String>,
}

impl Default for HttpClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpClientBuilder {
    const DEFAULT_MAX_REDIRECTS: usize = 5;

    fn redirect_policy(&self, policies: Arc<[Arc<dyn Policy>]>) -> redirect::Policy {
        let max_redirects = self.max_redirects;

        redirect::Policy::custom(move |attempt| {
            let rejection = policies.iter().find_map(|policy| {
                policy
                    .check_redirect(&RedirectContext::new(
                        attempt.status(),
                        attempt.url(),
                        attempt.previous(),
                    ))
                    .err()
                    .map(|denial| RedirectRejection::Denied {
                        policy: policy.name(),
                        reason: denial.into_reason(),
                    })
            });

            match rejection {
                Some(rejection) => attempt.error(rejection),
                None if attempt.previous().len() >= max_redirects => {
                    attempt.error(RedirectRejection::LimitExceeded)
                }
                None => attempt.follow(),
            }
        })
    }

    /// Creates a builder that permits everything and limits nothing.
    pub fn new() -> Self {
        Self {
            connect_timeout: None,
            read_timeout: None,
            request_timeout: None,
            default_headers: HeaderMap::new(),
            user_agent: None,
            max_redirects: Self::DEFAULT_MAX_REDIRECTS,
            max_response_bytes: None,
            concurrency: None,
            policies: Vec::new(),
            address_filter: None,
            middleware: Vec::new(),
            header_error: None,
        }
    }

    /// Sets the maximum time spent establishing a connection.
    pub fn connect_timeout(mut self, timeout: impl Into<Duration>) -> Self {
        self.connect_timeout = Some(timeout.into());
        self
    }

    /// Sets the maximum time between response body chunks.
    pub fn read_timeout(mut self, timeout: impl Into<Duration>) -> Self {
        self.read_timeout = Some(timeout.into());
        self
    }

    /// Sets a deadline for the whole request.
    ///
    /// Unset by default, because a whole-request deadline terminates a long-lived streaming
    /// response that is otherwise healthy.
    pub fn request_timeout(mut self, timeout: impl Into<Duration>) -> Self {
        self.request_timeout = Some(timeout.into());
        self
    }

    /// Sets the user agent sent with every request.
    pub fn user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = Some(user_agent.into());
        self
    }

    /// Adds a header sent with every request.
    ///
    /// A conversion failure is reported by
    /// [`build`](crate::client::builder::HttpClientBuilder::build).
    pub fn header(
        mut self,
        name: impl TryInto<HeaderName>,
        value: impl TryInto<HeaderValue>,
    ) -> Self {
        match (name.try_into(), value.try_into()) {
            (Ok(name), Ok(value)) => {
                self.default_headers.insert(name, value);
            }
            _ => {
                self.header_error = Some(String::from("invalid default header"));
            }
        }

        self
    }

    /// Adds headers sent with every request.
    pub fn headers(mut self, headers: impl Into<HeaderMap>) -> Self {
        self.default_headers.extend(headers.into());
        self
    }

    /// Sets how many redirects one request may follow.
    pub fn max_redirects(mut self, max: impl Into<usize>) -> Self {
        self.max_redirects = max.into();
        self
    }

    /// Sets how many response body bytes are accepted.
    pub fn max_response_bytes(mut self, max: impl Into<u64>) -> Self {
        self.max_response_bytes = Some(max.into());
        self
    }

    /// Limits how many requests this client may have in flight.
    pub fn concurrency_limit(mut self, permits: impl Into<usize>) -> Self {
        self.concurrency = Some(Arc::new(Semaphore::new(permits.into())));
        self
    }

    /// Shares an in-flight request budget with other clients.
    pub fn shared_concurrency(mut self, permits: impl Into<Arc<Semaphore>>) -> Self {
        self.concurrency = Some(permits.into());
        self
    }

    /// Adds a policy consulted on every request, redirect, and response head.
    pub fn policy(mut self, policy: impl Policy) -> Self {
        self.policies.push(Arc::new(policy));
        self
    }

    /// Sets the filter applied to every resolved address.
    pub fn address_filter(mut self, filter: impl AddressFilter) -> Self {
        self.address_filter = Some(Arc::new(filter));
        self
    }

    /// Adds a middleware applied after the policy and tracing middleware.
    pub fn middleware(mut self, middleware: impl Middleware) -> Self {
        self.middleware.push(Arc::new(middleware));
        self
    }

    /// Builds a client with its own connection pool.
    pub fn build(&self) -> Result<HttpClient, HttpClientError> {
        if let Some(message) = &self.header_error {
            return Err(HttpClientError::configuration(message));
        }

        let policies = Arc::<[Arc<dyn Policy>]>::from(self.policies.clone());
        let mut transport = Client::builder()
            .default_headers(self.default_headers.clone())
            .redirect(self.redirect_policy(policies.clone()))
            .referer(false)
            .no_proxy();

        if let Some(user_agent) = &self.user_agent {
            transport = transport.user_agent(user_agent);
        }

        if let Some(timeout) = self.connect_timeout {
            transport = transport.connect_timeout(timeout);
        }

        if let Some(timeout) = self.read_timeout {
            transport = transport.read_timeout(timeout);
        }

        if let Some(filter) = &self.address_filter {
            transport = transport.dns_resolver(Arc::new(GuardedResolver::new(filter.clone())));
        }

        let mut client = ClientBuilder::new(
            transport
                .build()
                .map_err(|error| HttpClientError::configuration(error.to_string()))?,
        )
        .with(PolicyMiddleware::new(policies))
        .with(TracingMiddleware::default());

        for middleware in &self.middleware {
            client = client.with_arc(middleware.clone());
        }

        Ok(
            HttpClient::from_inner(HttpClientInner::new(
                client.build(),
                Limits {
                    request_timeout: self.request_timeout,
                    max_response_bytes: self.max_response_bytes,
                    concurrency: self.concurrency.clone(),
                },
            ))
        )
    }
}
