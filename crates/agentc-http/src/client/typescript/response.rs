// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::cell::RefCell;

use agentc_executor_typescript::{
    guestjs::{
        errors::Error,
        host::{ClassSpec, HostClass},
        llrt::streams::HostReadableStream,
    },
    json::Json,
};
use bytes::{Bytes, BytesMut};
use futures::StreamExt;
use http::{HeaderMap, Method, StatusCode};
use url::Url;

use crate::client::{
    client::HttpClient,
    errors::HttpClientError,
    response::{HttpBodyStream, HttpResponse},
    typescript::{fetch::FetchRequest, headers::Headers},
};

/// The guest-visible response returned by the guest `fetch` function.
pub struct Response {
    status: StatusCode,
    url: Url,
    headers: HeaderMap,
    // Interior mutability so `body` is a property, as it is on the Web, while still being consumed
    // exactly once.
    body: RefCell<Option<HttpBodyStream>>,
}

impl Response {
    fn take_body(&self) -> Result<HttpBodyStream, Error> {
        self.body
            .borrow_mut()
            .take()
            .ok_or_else(|| Error::unexpected("agentc:http: the response body was already read"))
    }

    async fn collect(body: HttpBodyStream) -> Result<Bytes, Error> {
        let mut body = body;
        let mut collected = BytesMut::new();

        while let Some(chunk) = body.next().await {
            collected.extend_from_slice(&chunk?);
        }

        Ok(collected.freeze())
    }

    pub(crate) fn from_response(response: HttpResponse) -> Self {
        Self {
            status: response.status(),
            url: response.url().clone(),
            headers: response.headers().clone(),
            body: RefCell::new(Some(response.into_stream())),
        }
    }

    pub(crate) async fn send(client: &HttpClient, request: FetchRequest) -> Result<Self, Error> {
        let mut builder = client.request(
            request
                .init
                .method
                .as_deref()
                .map(Method::try_from)
                .transpose()
                .map_err(|_| HttpClientError::invalid_request("invalid method"))?
                .unwrap_or(Method::GET),
            &request.url,
        );

        if let Some(headers) = request.init.headers {
            for (name, value) in headers.0 {
                builder = builder.header(name.as_str(), value.as_str());
            }
        }

        if let Some(body) = request.body {
            builder = builder.body(Bytes::from(body));
        }

        Ok(Self::from_response(builder.send().await?))
    }
}

impl HostClass for Response {
    const NAME: &'static str = "Response";

    fn build(spec: &mut ClassSpec<Self>) {
        spec.getter("status", |response, _scope| Ok(response.status.as_u16()));

        spec.getter("ok", |response, _scope| Ok(response.status.is_success()));

        spec.getter("url", |response, _scope| Ok(response.url.to_string()));

        spec.getter("bodyUsed", |response, _scope| Ok(response.body.borrow().is_none()));

        spec.getter("statusText", |response, _scope| {
            Ok(response
                .status
                .canonical_reason()
                .unwrap_or_default()
                .to_owned())
        });

        spec.getter("headers", |response, _scope| Ok(Headers::new(response.headers.clone())));

        spec.getter("body", |response, _scope| {
            Ok(HostReadableStream::from_stream(
                response
                    .take_body()?
                    .map(|chunk| chunk.map_err(Error::from)),
            ))
        });

        spec.async_method("text", |response, _scope, _args| {
            let body = response.take_body()?;

            Ok(async move { Ok(String::from_utf8_lossy(&Self::collect(body).await?).into_owned()) })
        });

        spec.async_method("json", |response, _scope, _args| {
            let body = response.take_body()?;

            Ok(async move {
                serde_json::from_slice::<serde_json::Value>(&Self::collect(body).await?)
                    .map(Json)
                    .map_err(|error| Error::unexpected(format!("agentc:http: {error}")))
            })
        });

        spec.async_method("bytes", |response, _scope, _args| {
            let body = response.take_body()?;

            Ok(async move { Self::collect(body).await })
        });

        spec.async_method("arrayBuffer", |response, _scope, _args| {
            let body = response.take_body()?;

            Ok(async move { Self::collect(body).await })
        });
    }
}
