// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::{borrow::Cow, net::SocketAddr};

use http::{HeaderMap, Method, StatusCode};
use url::Url;

/// A refusal produced by a [`Policy`](crate::client::policy::Policy).
#[derive(Debug, Clone)]
pub struct Denied {
    reason: Cow<'static, str>,
}

impl Denied {
    /// Builds a refusal carrying the given reason.
    pub fn new(reason: impl Into<Cow<'static, str>>) -> Self {
        Self {
            reason: reason.into(),
        }
    }

    /// The reason this refusal carries.
    pub fn reason(&self) -> &str {
        &self.reason
    }

    pub(crate) fn into_reason(self) -> String {
        self.reason.into_owned()
    }
}

/// An outgoing request offered to a [`Policy`](crate::client::policy::Policy).
pub struct RequestContext<'a> {
    method: &'a Method,
    url: &'a Url,
    headers: &'a HeaderMap,
}

impl<'a> RequestContext<'a> {
    pub(crate) fn new(method: &'a Method, url: &'a Url, headers: &'a HeaderMap) -> Self {
        Self {
            method,
            url,
            headers,
        }
    }

    /// The request method.
    pub fn method(&self) -> &Method {
        self.method
    }

    /// The request destination.
    pub fn url(&self) -> &Url {
        self.url
    }

    /// The request headers.
    pub fn headers(&self) -> &HeaderMap {
        self.headers
    }
}

/// A redirect hop offered to a [`Policy`](crate::client::policy::Policy) before it is followed.
pub struct RedirectContext<'a> {
    status: StatusCode,
    url: &'a Url,
    previous: &'a [Url],
}

impl<'a> RedirectContext<'a> {
    pub(crate) fn new(status: StatusCode, url: &'a Url, previous: &'a [Url]) -> Self {
        Self {
            status,
            url,
            previous,
        }
    }

    /// The status that produced this hop.
    pub fn status(&self) -> StatusCode {
        self.status
    }

    /// The destination of this hop.
    pub fn url(&self) -> &Url {
        self.url
    }

    /// The destinations already visited, oldest first.
    pub fn previous(&self) -> &[Url] {
        self.previous
    }
}

/// A response head offered to a [`Policy`](crate::client::policy::Policy) before its body is read.
pub struct ResponseContext<'a> {
    status: StatusCode,
    url: &'a Url,
    headers: &'a HeaderMap,
    content_length: Option<u64>,
}

impl<'a> ResponseContext<'a> {
    pub(crate) fn new(
        status: StatusCode,
        url: &'a Url,
        headers: &'a HeaderMap,
        content_length: Option<u64>,
    ) -> Self {
        Self {
            status,
            url,
            headers,
            content_length,
        }
    }

    /// The response status.
    pub fn status(&self) -> StatusCode {
        self.status
    }

    /// The destination that produced this response, after any redirects.
    pub fn url(&self) -> &Url {
        self.url
    }

    /// The response headers.
    pub fn headers(&self) -> &HeaderMap {
        self.headers
    }

    /// The advertised body length, when the response declares one.
    pub fn content_length(&self) -> Option<u64> {
        self.content_length
    }
}

/// A rule consulted before a request is sent, before a redirect is followed, and when a response
/// head arrives.
///
/// Implementing [`check_url`](crate::client::policy::Policy::check_url) alone is enough for a
/// destination rule. The request and redirect hooks delegate to it by default, so a destination
/// rule cannot be bypassed by a redirect.
pub trait Policy: Send + Sync + 'static {
    /// The name reported when this policy denies.
    fn name(&self) -> &'static str;

    /// Evaluates a destination.
    fn check_url(&self, _url: &Url) -> Result<(), Denied> {
        Ok(())
    }

    /// Evaluates an outgoing request.
    fn check_request(&self, request: &RequestContext<'_>) -> Result<(), Denied> {
        self.check_url(request.url())
    }

    /// Evaluates a redirect hop before it is followed.
    fn check_redirect(&self, redirect: &RedirectContext<'_>) -> Result<(), Denied> {
        self.check_url(redirect.url())
    }

    /// Evaluates a response head before its body is read.
    fn check_response(&self, _response: &ResponseContext<'_>) -> Result<(), Denied> {
        Ok(())
    }

    /// Evaluates an address a host name resolved to.
    fn check_address(&self, _host: &str, _address: SocketAddr) -> Result<(), Denied> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DestinationPolicy;

    impl Policy for DestinationPolicy {
        fn name(&self) -> &'static str {
            "destination"
        }

        fn check_url(&self, url: &Url) -> Result<(), Denied> {
            match url.host_str() {
                Some("allowed.test") => Ok(()),
                _ => Err(Denied::new("host is not permitted")),
            }
        }
    }

    fn url(value: &str) -> Url {
        Url::parse(value).expect("test url parses")
    }

    #[test]
    fn destination_rule_covers_the_initial_request() {
        assert!(
            DestinationPolicy
                .check_request(&RequestContext::new(
                    &Method::GET,
                    &url("https://denied.test/"),
                    &HeaderMap::new(),
                ))
                .is_err()
        );
    }

    #[test]
    fn destination_rule_covers_every_redirect_hop() {
        assert!(
            DestinationPolicy
                .check_redirect(&RedirectContext::new(
                    StatusCode::FOUND,
                    &url("https://denied.test/"),
                    &[url("https://allowed.test/")],
                ))
                .is_err()
        );
    }

    #[test]
    fn unrelated_hooks_allow_by_default() {
        assert!(
            DestinationPolicy
                .check_response(&ResponseContext::new(
                    StatusCode::OK,
                    &url("https://allowed.test/"),
                    &HeaderMap::new(),
                    None,
                ))
                .is_ok()
        );
    }
}
