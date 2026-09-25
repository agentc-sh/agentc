// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::borrow::Cow;

use agentc_executor_python::{
    backend::ExecutorBackend,
    guestpy::{errors::Error, host::module::ModuleSpec},
};

use crate::client::{
    client::HttpClient,
    python::{
        body::{Form, Json},
        client::Client,
        exceptions::{
            BodyClosed, BodyConsumed, BodyError, DecodingError, HttpError, HttpStatusError,
            InvalidRequest, RequestDenied, RequestError, ResponseError, ResponseTooLarge,
            TimeoutException, TooManyRedirects, TransportError,
        },
        headers::Headers,
        pending::PendingResponse,
        request::Request,
        response::{Response, ResponseBody},
    },
};

pub struct HttpModule {
    name: Cow<'static, str>,
    client: HttpClient,
}

impl HttpModule {
    const DEFAULT_NAME: &'static str = "agentc_http";

    pub fn new(client: impl Into<HttpClient>) -> Self {
        Self {
            name: Cow::Borrowed(Self::DEFAULT_NAME),
            client: client.into(),
        }
    }

    pub fn with_name(mut self, name: impl Into<Cow<'static, str>>) -> Self {
        self.name = name.into();
        self
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn client(&self) -> &HttpClient {
        &self.client
    }
}

impl<B: ExecutorBackend> TryFrom<HttpModule> for ModuleSpec<B> {
    type Error = Error;

    fn try_from(module: HttpModule) -> Result<Self, Self::Error> {
        Ok(ModuleSpec::new(module.name.clone())
            .state(module)
            .exception_type::<HttpError>()
            .exception_type::<RequestError<B>>()
            .exception_type::<TransportError<B>>()
            .exception_type::<TimeoutException<B>>()
            .exception_type::<RequestDenied<B>>()
            .exception_type::<TooManyRedirects<B>>()
            .exception_type::<InvalidRequest<B>>()
            .exception_type::<ResponseError<B>>()
            .exception_type::<HttpStatusError<B>>()
            .exception_type::<DecodingError<B>>()
            .exception_type::<ResponseTooLarge<B>>()
            .exception_type::<BodyError>()
            .exception_type::<BodyConsumed>()
            .exception_type::<BodyClosed>()
            .class::<Headers<B>>()?
            .class::<Json>()?
            .class::<Form>()?
            .class::<Request<B>>()?
            .class::<Response<B>>()?
            .class::<ResponseBody<B>>()?
            .class::<PendingResponse<B>>()?
            .class::<Client<B>>()?)
    }
}
