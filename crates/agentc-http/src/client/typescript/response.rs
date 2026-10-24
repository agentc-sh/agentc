// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::{cell::RefCell, marker::PhantomData};

use agentc_executor_typescript::{
    backend::ExecutorBackend,
    guestjs::{backend::Backend, errors::Error, host::stream::StreamSource, host_class},
    json::Json,
};
use bytes::{Bytes, BytesMut};
use futures::{FutureExt, Stream, StreamExt};
use http::{HeaderMap, Method, StatusCode};
use url::Url;

use crate::client::{
    client::HttpClient,
    errors::HttpClientError,
    response::{HttpBodyStream, HttpResponse},
    typescript::{fetch::FetchRequest, headers::Headers},
};

/// The guest-visible response returned by the guest `fetch` function.
pub struct Response<B: Backend> {
    status: StatusCode,
    url: Url,
    headers: HeaderMap,
    // Interior mutability so `body` is a property, as it is on the Web, while still being consumed
    // exactly once.
    body: RefCell<Option<HttpBodyStream>>,
    _marker: PhantomData<fn() -> B>,
}

impl<B: Backend> Response<B> {
    pub(crate) fn from_response(response: HttpResponse) -> Self {
        Self {
            status: response.status(),
            url: response.url().clone(),
            headers: response.headers().clone(),
            body: RefCell::new(Some(response.into_stream())),
            _marker: PhantomData,
        }
    }

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

        if let Some(body) = request.init.body {
            builder = builder.body(Bytes::from(body));
        }

        Ok(Self::from_response(builder.send().await?))
    }
}

#[host_class(rename_all = "camelCase", backend = B, crate_path = agentc_executor_typescript::guestjs)]
impl<B: ExecutorBackend> Response<B> {
    #[guestjs(get)]
    fn status(&self) -> Result<u16, Error> {
        Ok(self.status.as_u16())
    }

    #[guestjs(get)]
    fn ok(&self) -> Result<bool, Error> {
        Ok(self.status.is_success())
    }

    #[guestjs(get)]
    fn url(&self) -> Result<String, Error> {
        Ok(self.url.to_string())
    }

    #[guestjs(get)]
    fn body_used(&self) -> Result<bool, Error> {
        Ok(self.body.borrow().is_none())
    }

    #[guestjs(get)]
    fn status_text(&self) -> Result<String, Error> {
        Ok(
            self.status
                .canonical_reason()
                .unwrap_or_default()
                .to_owned()
        )
    }

    #[guestjs(get)]
    fn headers(&self) -> Result<Headers, Error> {
        Ok(Headers::new(self.headers.clone()))
    }

    #[guestjs(get)]
    fn body(
        &self,
    ) -> Result<StreamSource<impl Stream<Item = Result<Bytes, Error>> + use<B>>, Error> {
        Ok(
            StreamSource::new(
                self.take_body()?
                    .map(|chunk| chunk.map_err(Error::from)),
            )
        )
    }

    #[guestjs(async_method)]
    fn text(&self) -> Result<impl Future<Output = Result<String, Error>> + use<B>, Error> {
        Ok(
            Self::collect(self.take_body()?)
                .map(|body| body.map(|bytes| String::from_utf8_lossy(&bytes).into_owned()))
        )
    }

    #[guestjs(async_method)]
    fn json(&self) -> Result<impl Future<Output = Result<Json, Error>> + use<B>, Error> {
        Ok(
            Self::collect(self.take_body()?)
                .map(|body| {
                    serde_json::from_slice::<serde_json::Value>(&body?)
                        .map(Json)
                        .map_err(|error| Error::unexpected(format!("agentc:http: {error}")))
                })
        )
    }

    #[guestjs(async_method)]
    fn bytes(&self) -> Result<impl Future<Output = Result<Bytes, Error>> + use<B>, Error> {
        Ok(Self::collect(self.take_body()?))
    }

    #[guestjs(async_method)]
    fn array_buffer(&self) -> Result<impl Future<Output = Result<Bytes, Error>> + use<B>, Error> {
        Ok(Self::collect(self.take_body()?))
    }
}
