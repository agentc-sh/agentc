// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

pub mod builder;
pub mod client;
pub mod errors;
pub mod policy;
pub mod request;
pub mod response;

mod limits;
mod middleware;
mod resolver;

pub use crate::client::{
    builder::HttpClientBuilder,
    client::HttpClient,
    errors::HttpClientError,
    policy::{AddressFilter, Denied, Policy, RedirectContext, RequestContext, ResponseContext},
    request::{HttpRequest, HttpRequestBuilder},
    response::{HttpBodyStream, HttpResponse},
};
