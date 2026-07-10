// SPDX-FileCopyrightText: 2026 Timothy Pogue
//
// SPDX-License-Identifier: LicenseRef-Proprietary

use std::{
    future::{Future, IntoFuture},
    pin::Pin,
    time::Duration,
};

use reqwest::{
    Method,
    Response,
    header::{HeaderMap, HeaderValue, IntoHeaderName, InvalidHeaderValue},
};
use reqwest_middleware::RequestBuilder;
use serde::{Serialize, de::DeserializeOwned};

use crate::client::{base::BaseClient, errors::A2aClientError};


/// Per-call request extras applied to the outgoing [`RequestBuilder`] just before send.
///
/// Headers and the timeout are applied first; closures registered via `.with(...)`
/// are then applied in registration order so they can override the typed extras
/// when needed.
pub struct Request<'a> {
    headers: HeaderMap,
    timeout: Option<Duration>,
    modifiers: Vec<Box<dyn FnOnce(RequestBuilder) -> RequestBuilder + Send + 'a>>,
}

impl<'a> Default for Request<'a> {
    fn default() -> Self {
        Self {
            headers: HeaderMap::new(),
            timeout: None,
            modifiers: Vec::new(),
        }
    }
}

impl<'a> Request<'a> {
    pub fn header(mut self, key: impl IntoHeaderName, value: impl TryInto<HeaderValue, Error = InvalidHeaderValue>) -> Result<Self, A2aClientError> {
        self.headers
            .insert(key, {
                value
                    .try_into()
                    .map_err(|e| A2aClientError::configuration(e.to_string()))?
            });
        Ok(self)
    }

    pub fn header_lossy(mut self, key: impl IntoHeaderName, value: impl TryInto<HeaderValue, Error = InvalidHeaderValue>) -> Self {
        if let Ok(value) = value.try_into() {
            self.headers.insert(key, value);
        }
        self
    }

    pub fn timeout(mut self, duration: Duration) -> Self {
        self.timeout = Some(duration);
        self
    }

    pub fn with<F>(mut self, f: F) -> Self
    where
        F: FnOnce(RequestBuilder) -> RequestBuilder + Send + 'a,
    {
        self.modifiers.push(Box::new(f));
        self
    }

    pub(crate) fn apply(self, req: RequestBuilder) -> RequestBuilder {
        let req = self.headers
            .into_iter()
            .fold(req, |r, (k, v)| {
                if let Some(k) = k { r.header(k, v) } else { r }
            });

        let req = if let Some(t) = self.timeout {
            req.timeout(t)
        } else {
            req
        };

        self.modifiers
            .into_iter()
            .fold(req, |r, f| f(r))
    }
}


// The one piece that differs between transports: turns the prepared request into
// the intermediate value `I` (a `Response` for HTTP, a `WebSocket` for an upgrade).
type Execute<'a, I> =
    Box<
        dyn FnOnce(&'a BaseClient, RequestBuilder)
            -> Pin<Box<dyn Future<Output = Result<I, A2aClientError>> + Send + 'a>>
        + Send
        + 'a,
    >;

// Turns the intermediate value `I` into the call's output `T`. Defaults to identity.
type Mapper<'a, I, T> =
    Box<
        dyn FnOnce(I)
            -> Pin<Box<dyn Future<Output = Result<T, A2aClientError>> + Send + 'a>>
        + Send
        + 'a,
    >;


/// An awaitable request builder. Awaiting it sends the request; there is no
/// separate `.send()` step.
///
/// Chain `.header(...)`, `.timeout(...)`, or `.with(...)` to customize a single
/// request before awaiting. `I` is the transport value ([`Response`] or
/// [`WebSocket`]); `T` is the awaited output, defaulting to `I`.
pub struct Call<'a, I, T = I> {
    client: &'a BaseClient,
    method: Method,
    url: String,
    body: Option<Result<Vec<u8>, A2aClientError>>,
    query: Option<Result<String, A2aClientError>>,
    request: Request<'a>,
    execute: Execute<'a, I>,
    mapper: Mapper<'a, I, T>,
}

// NOTE: Leave identity_mapper as a free floating function
fn identity_mapper<'a, I: Send + 'a>() -> Mapper<'a, I, I> {
    Box::new(|value| Box::pin(async move { Ok(value) }))
}

impl<'a> Call<'a, Response> {
    fn http(client: &'a BaseClient, method: Method, url: impl Into<String>) -> Self {
        Self {
            client,
            method,
            url: url.into(),
            body: None,
            query: None,
            request: Request::default(),
            execute: Box::new(|client, builder| Box::pin(client.send(builder))),
            mapper: identity_mapper(),
        }
    }

    pub(crate) fn get(client: &'a BaseClient, url: impl Into<String>) -> Self {
        Self::http(client, Method::GET, url)
    }

    pub(crate) fn post(client: &'a BaseClient, url: impl Into<String>) -> Self {
        Self::http(client, Method::POST, url)
    }

    pub(crate) fn put(client: &'a BaseClient, url: impl Into<String>) -> Self {
        Self::http(client, Method::PUT, url)
    }

    pub(crate) fn delete(client: &'a BaseClient, url: impl Into<String>) -> Self {
        Self::http(client, Method::DELETE, url)
    }
}

impl<'a, I, T> Call<'a, I, T> {
    pub fn header(mut self, key: impl IntoHeaderName, value: impl TryInto<HeaderValue, Error = InvalidHeaderValue>) -> Result<Self, A2aClientError> {
        self.request = self.request.header(key, value)?;
        Ok(self)
    }

    pub fn header_lossy(mut self, key: impl IntoHeaderName, value: impl TryInto<HeaderValue, Error = InvalidHeaderValue>) -> Self {
        self.request = self.request.header_lossy(key, value);
        self
    }

    pub fn maybe_header(mut self, key: impl IntoHeaderName, value: Option<impl TryInto<HeaderValue, Error = InvalidHeaderValue>>) -> Result<Self, A2aClientError> {
        if let Some(value) = value {
            self.request = self.request.header(key, value)?;
        }
        Ok(self)
    }

    pub fn maybe_header_lossy(mut self, key: impl IntoHeaderName, value: Option<impl TryInto<HeaderValue, Error = InvalidHeaderValue>>) -> Self {
        if let Some(value) = value {
            self.request = self.request.header_lossy(key, value);
        }
        self
    }

    pub fn timeout(mut self, duration: Duration) -> Self {
        self.request = self.request.timeout(duration);
        self
    }

    pub fn with<F>(mut self, f: F) -> Self
    where
        F: FnOnce(RequestBuilder) -> RequestBuilder + Send + 'a,
    {
        self.request = self.request.with(f);
        self
    }

    /// Sets the JSON request body.
    pub fn body<B: Serialize>(mut self, body: &B) -> Self {
        // Serialized here; any failure is surfaced when the call is awaited.
        self.body = Some(serde_json::to_vec(body).map_err(A2aClientError::serialize));
        self
    }

    /// Sets the query parameters.
    pub fn params<P: Serialize>(mut self, params: &P) -> Self {
        self.query = Some(
            serde_qs::Config::new()
                .max_depth(5)
                .use_form_encoding(false)
                .array_format(serde_qs::ArrayFormat::EmptyIndexed)
                .serialize_string(params)
                .map_err(|e| A2aClientError::configuration(e.to_string())),
        );
        self
    }

    /// Conditionally sets the query parameters if `Some`, does nothing if `None`.
    pub fn maybe_params<P: Serialize>(self, params: Option<&P>) -> Self {
        if let Some(p) = params {
            self.params(p)
        } else {
            self
        }
    }
}

impl<'a, I> Call<'a, I, I>
where
    I: Send + 'a,
{
    /// Maps the response into the call's output type.
    pub fn map<U, F, Fut>(self, f: F) -> Call<'a, I, U>
    where
        F: FnOnce(I) -> Fut + Send + 'a,
        Fut: Future<Output = Result<U, A2aClientError>> + Send + 'a,
    {
        Call {
            client: self.client,
            method: self.method,
            url: self.url,
            body: self.body,
            query: self.query,
            request: self.request,
            execute: self.execute,
            mapper: Box::new(move |value| Box::pin(f(value))),
        }
    }
}

impl<'a> Call<'a, Response, Response> {
    /// Deserializes the response body as JSON.
    pub fn json<U: DeserializeOwned + Send + 'a>(self) -> Call<'a, Response, U> {
        self.map(|response| async move {
            let bytes = response
                .bytes()
                .await
                .map_err(|e| A2aClientError::request(reqwest_middleware::Error::Reqwest(e)))?;

            // An empty body is treated as JSON null so unit-like responses decode.
            if bytes.is_empty() {
                serde_json::from_value(serde_json::Value::Null).map_err(A2aClientError::serialize)
            } else {
                serde_json::from_slice(&bytes).map_err(A2aClientError::serialize)
            }
        })
    }

    /// Returns the response body as text.
    pub fn text(self) -> Call<'a, Response, String> {
        self.map(|response| async move {
            response
                .text()
                .await
                .map_err(|e| A2aClientError::request(reqwest_middleware::Error::Reqwest(e)))
        })
    }

    /// Returns the response body as bytes.
    pub fn bytes(self) -> Call<'a, Response, Vec<u8>> {
        self.map(|response| async move {
            response
                .bytes()
                .await
                .map(|b| b.to_vec())
                .map_err(|e| A2aClientError::request(reqwest_middleware::Error::Reqwest(e)))
        })
    }

    /// Returns a unit type, ignoring the response body.
    pub fn empty(self) -> Call<'a, Response, ()> {
        self.map(|response| async move {
            // Consume the body to ensure the connection can be reused, but discard it.
            response
                .bytes()
                .await
                .map_err(|e| A2aClientError::request(reqwest_middleware::Error::Reqwest(e)))
                .map(|_| ())
        })
    }
}

impl<'a, I, T> IntoFuture for Call<'a, I, T>
where
    I: Send + 'a,
    T: Send + 'a,
{
    type Output = Result<T, A2aClientError>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send + 'a>>;

    fn into_future(self) -> Self::IntoFuture {
        let Call {
            client,
            method,
            url,
            body,
            query,
            request,
            execute,
            mapper,
        } = self;

        Box::pin(async move {
            let mut builder = client.request(method, &url, query.transpose()?.as_deref());

            if let Some(body) = body {
                builder = builder
                    .body(body?);
            }

            mapper(execute(client, request.apply(builder)).await?).await
        })
    }
}
