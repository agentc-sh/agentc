// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use agentc_executor_python::{
    backend::ExecutorBackend,
    guestpy::{HostException, handle::Instance},
};

use crate::client::python::{request::Request, response::Response};

#[derive(HostException)]
#[guestpy(name = "HTTPError", crate_path = agentc_executor_python::guestpy)]
pub struct HttpError {
    #[guestpy(arg)]
    pub(crate) message: String,
}

#[derive(HostException)]
#[guestpy(base = "HttpError", crate_path = agentc_executor_python::guestpy)]
pub struct RequestError<B: ExecutorBackend> {
    #[guestpy(arg)]
    pub(crate) message: String,
    pub(crate) request: Instance<B, Request<B>>,
}

#[derive(HostException)]
#[guestpy(base = "RequestError<B>", crate_path = agentc_executor_python::guestpy)]
pub struct TransportError<B: ExecutorBackend> {
    #[guestpy(arg)]
    pub(crate) message: String,
    pub(crate) request: Instance<B, Request<B>>,
}

#[derive(HostException)]
#[guestpy(base = "TransportError<B>", crate_path = agentc_executor_python::guestpy)]
pub struct TimeoutException<B: ExecutorBackend> {
    #[guestpy(arg)]
    pub(crate) message: String,
    pub(crate) request: Instance<B, Request<B>>,
}

#[derive(HostException)]
#[guestpy(base = "RequestError<B>", crate_path = agentc_executor_python::guestpy)]
pub struct RequestDenied<B: ExecutorBackend> {
    #[guestpy(arg)]
    pub(crate) message: String,
    pub(crate) request: Instance<B, Request<B>>,
    pub(crate) policy: String,
    pub(crate) reason: String,
}

#[derive(HostException)]
#[guestpy(base = "RequestError<B>", crate_path = agentc_executor_python::guestpy)]
pub struct TooManyRedirects<B: ExecutorBackend> {
    #[guestpy(arg)]
    pub(crate) message: String,
    pub(crate) request: Instance<B, Request<B>>,
}

#[derive(HostException)]
#[guestpy(base = "RequestError<B>", crate_path = agentc_executor_python::guestpy)]
pub struct InvalidRequest<B: ExecutorBackend> {
    #[guestpy(arg)]
    pub(crate) message: String,
    pub(crate) request: Instance<B, Request<B>>,
}

#[derive(HostException)]
#[guestpy(base = "HttpError", crate_path = agentc_executor_python::guestpy)]
pub struct ResponseError<B: ExecutorBackend> {
    #[guestpy(arg)]
    pub(crate) message: String,
    pub(crate) response: Instance<B, Response<B>>,
}

#[derive(HostException)]
#[guestpy(
    name = "HTTPStatusError",
    base = "ResponseError<B>",
    crate_path = agentc_executor_python::guestpy
)]
pub struct HttpStatusError<B: ExecutorBackend> {
    #[guestpy(arg)]
    pub(crate) message: String,
    pub(crate) response: Instance<B, Response<B>>,
}

#[derive(HostException)]
#[guestpy(base = "ResponseError<B>", crate_path = agentc_executor_python::guestpy)]
pub struct DecodingError<B: ExecutorBackend> {
    #[guestpy(arg)]
    pub(crate) message: String,
    pub(crate) response: Instance<B, Response<B>>,
}

#[derive(HostException)]
#[guestpy(base = "ResponseError<B>", crate_path = agentc_executor_python::guestpy)]
pub struct ResponseTooLarge<B: ExecutorBackend> {
    #[guestpy(arg)]
    pub(crate) message: String,
    pub(crate) response: Instance<B, Response<B>>,
    pub(crate) limit: u64,
}

#[derive(HostException)]
#[guestpy(builtin = "RuntimeError", crate_path = agentc_executor_python::guestpy)]
pub struct BodyError {
    #[guestpy(arg)]
    pub(crate) message: String,
}

#[derive(HostException)]
#[guestpy(base = "BodyError", crate_path = agentc_executor_python::guestpy)]
pub struct BodyConsumed {
    #[guestpy(arg)]
    pub(crate) message: String,
}

#[derive(HostException)]
#[guestpy(base = "BodyError", crate_path = agentc_executor_python::guestpy)]
pub struct BodyClosed {
    #[guestpy(arg)]
    pub(crate) message: String,
}
