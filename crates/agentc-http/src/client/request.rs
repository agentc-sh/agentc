// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::{borrow::Cow, sync::Arc, time::Duration};

use bytes::Bytes;
use http::{HeaderMap, HeaderName, HeaderValue, Method, header::CONTENT_TYPE};
use serde::Serialize;
use url::Url;

use crate::client::{client::HttpClientInner, errors::HttpClientError, response::HttpResponse};

/// A prepared request.
#[derive(Clone)]
pub struct HttpRequest {
    method: Method,
    url: Url,
    headers: HeaderMap,
    body: Option<Bytes>,
    timeout: Option<Duration>,
    label: Option<Cow<'static, str>>,
}

impl HttpRequest {
    /// Creates a request with no headers and no body.
    pub fn new(method: impl Into<Method>, url: impl Into<Url>) -> Self {
        Self {
            method: method.into(),
            url: url.into(),
            headers: HeaderMap::new(),
            body: None,
            timeout: None,
            label: None,
        }
    }

    /// The request method.
    pub fn method(&self) -> &Method {
        &self.method
    }

    /// The request destination.
    pub fn url(&self) -> &Url {
        &self.url
    }

    /// The request headers.
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    /// The request headers, mutably.
    pub fn headers_mut(&mut self) -> &mut HeaderMap {
        &mut self.headers
    }

    /// The request body.
    pub fn body(&self) -> Option<&Bytes> {
        self.body.as_ref()
    }

    pub(crate) fn into_parts(self) -> HttpRequestParts {
        HttpRequestParts {
            method: self.method,
            url: self.url,
            headers: self.headers,
            body: self.body,
            timeout: self.timeout,
            label: self.label,
        }
    }
}

pub(crate) struct HttpRequestParts {
    pub(crate) method: Method,
    pub(crate) url: Url,
    pub(crate) headers: HeaderMap,
    pub(crate) body: Option<Bytes>,
    pub(crate) timeout: Option<Duration>,
    pub(crate) label: Option<Cow<'static, str>>,
}

/// Builds and sends one request.
///
/// A failure is deferred to [`send`](crate::client::request::HttpRequestBuilder::send) so a chain
/// reads without an intermediate result on every line.
pub struct HttpRequestBuilder {
    client: Arc<HttpClientInner>,
    request: Result<HttpRequest, HttpClientError>,
}

impl HttpRequestBuilder {
    pub(crate) fn new(client: Arc<HttpClientInner>, method: Method, url: &str) -> Self {
        Self {
            client,
            request: Url::parse(url)
                .map(|url| HttpRequest::new(method, url))
                .map_err(|error| HttpClientError::invalid_request(error.to_string())),
        }
    }

    /// Adds a header.
    ///
    /// A conversion failure is reported by
    /// [`send`](crate::client::request::HttpRequestBuilder::send).
    pub fn header(self, name: impl TryInto<HeaderName>, value: impl TryInto<HeaderValue>) -> Self {
        self.map(|mut request| match (name.try_into(), value.try_into()) {
            (Ok(name), Ok(value)) => {
                request.headers.insert(name, value);

                Ok(request)
            }
            _ => Err(HttpClientError::invalid_request("invalid header")),
        })
    }

    /// Adds headers.
    pub fn headers(self, headers: impl Into<HeaderMap>) -> Self {
        self.map(|mut request| {
            request.headers.extend(headers.into());

            Ok(request)
        })
    }

    /// Sets the query string from a serializable value.
    pub fn query<T>(self, query: &T) -> Self
    where
        T: Serialize + ?Sized,
    {
        self.map(|mut request| {
            request
                .url
                .set_query(Some(
                    &serde_urlencoded::to_string(query)
                        .map_err(|error| HttpClientError::invalid_request(error.to_string()))?,
                ));

            Ok(request)
        })
    }

    /// Sets the request body.
    pub fn body(self, body: impl Into<Bytes>) -> Self {
        self.map(|mut request| {
            request.body = Some(body.into());

            Ok(request)
        })
    }

    /// Sets a JSON request body and the matching content type.
    pub fn json<T>(self, body: &T) -> Self
    where
        T: Serialize + ?Sized,
    {
        self.map(|mut request| {
            request
                .headers
                .insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
            request.body = Some(Bytes::from(
                serde_json::to_vec(body)
                    .map_err(|error| HttpClientError::invalid_request(error.to_string()))?,
            ));

            Ok(request)
        })
    }

    /// Overrides the client's whole-request deadline for this request.
    pub fn timeout(self, timeout: impl Into<Duration>) -> Self {
        self.map(|mut request| {
            request.timeout = Some(timeout.into());

            Ok(request)
        })
    }

    /// Names the trace span emitted for this request.
    pub fn label(self, label: impl Into<Cow<'static, str>>) -> Self {
        self.map(|mut request| {
            request.label = Some(label.into());

            Ok(request)
        })
    }

    /// Sends the request.
    pub async fn send(self) -> Result<HttpResponse, HttpClientError> {
        self.client.execute(self.request?).await
    }

    fn map<F>(mut self, f: F) -> Self
    where
        F: FnOnce(HttpRequest) -> Result<HttpRequest, HttpClientError>,
    {
        self.request = self.request.and_then(f);
        self
    }
}
