// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::{
    pin::Pin,
    task::{Context, Poll},
};

use bytes::{Bytes, BytesMut};
use futures::{Stream, StreamExt, stream::BoxStream};
use http::{HeaderMap, StatusCode};
use reqwest::Response;
use serde::de::DeserializeOwned;
use tokio::sync::OwnedSemaphorePermit;
use url::Url;

use crate::client::errors::HttpClientError;

/// A response whose head has arrived and whose body has not been read.
pub struct HttpResponse {
    status: StatusCode,
    url: Url,
    headers: HeaderMap,
    body: HttpBodyStream,
}

impl HttpResponse {
    pub(crate) fn new(
        response: Response,
        limit: Option<u64>,
        permit: Option<OwnedSemaphorePermit>,
    ) -> Self {
        Self {
            status: response.status(),
            url: response.url().clone(),
            headers: response.headers().clone(),
            body: HttpBodyStream::new(response, limit, permit),
        }
    }

    /// The response status.
    pub fn status(&self) -> StatusCode {
        self.status
    }

    /// Whether the status is in the success range.
    pub fn is_success(&self) -> bool {
        self.status.is_success()
    }

    /// The destination that produced this response, after any redirects.
    pub fn url(&self) -> &Url {
        &self.url
    }

    /// The response headers.
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    /// Reads the whole body.
    pub async fn bytes(self) -> Result<Bytes, HttpClientError> {
        let mut body = self.body;
        let mut collected = BytesMut::new();

        while let Some(chunk) = body.next().await {
            collected.extend_from_slice(&chunk?);
        }

        Ok(collected.freeze())
    }

    /// Reads the whole body as text.
    pub async fn text(self) -> Result<String, HttpClientError> {
        Ok(String::from_utf8_lossy(&self.bytes().await?).into_owned())
    }

    /// Reads the whole body and deserializes it as JSON.
    pub async fn json<T>(self) -> Result<T, HttpClientError>
    where
        T: DeserializeOwned,
    {
        serde_json::from_slice(&self.bytes().await?)
            .map_err(|error| HttpClientError::decode(error.to_string()))
    }

    /// Consumes the response and yields its body as it arrives.
    pub fn into_stream(self) -> HttpBodyStream {
        self.body
    }
}

/// A response body streamed as it arrives.
///
/// The stream enforces the client's response size limit, yielding
/// [`BodyTooLarge`](crate::client::errors::HttpClientError::BodyTooLarge) rather than truncating.
pub struct HttpBodyStream {
    inner: BoxStream<'static, Result<Bytes, HttpClientError>>,
    limit: Option<u64>,
    read: u64,
    // Held so an in-flight request keeps its concurrency permit until its body is finished.
    _permit: Option<OwnedSemaphorePermit>,
}

impl HttpBodyStream {
    fn new(response: Response, limit: Option<u64>, permit: Option<OwnedSemaphorePermit>) -> Self {
        Self {
            inner: response
                .bytes_stream()
                .map(|chunk| {
                    chunk.map_err(|error| HttpClientError::Transport {
                        source: error.into(),
                    })
                })
                .boxed(),
            limit,
            read: 0,
            _permit: permit,
        }
    }
}

impl Stream for HttpBodyStream {
    type Item = Result<Bytes, HttpClientError>;

    fn poll_next(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let stream = self.get_mut();

        match stream.inner.as_mut().poll_next(context) {
            Poll::Ready(Some(Ok(chunk))) => {
                stream.read += chunk.len() as u64;

                match stream.limit {
                    Some(limit) if stream.read > limit => {
                        Poll::Ready(Some(Err(HttpClientError::BodyTooLarge { limit })))
                    }
                    _ => Poll::Ready(Some(Ok(chunk))),
                }
            }
            other => other,
        }
    }
}
