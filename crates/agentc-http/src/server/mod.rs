// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

pub mod dto;
pub mod errors;
pub mod extractors;
pub mod openapi;
pub mod server;
pub mod state;
pub mod stream;

pub use crate::server::{
    errors::ApiError,
    openapi::OpenApiRouterExt,
    server::{HttpServer, HttpServerBuilder, merge_routers},
    state::DefaultTenantId,
    stream::CancelOnDropStream,
};

#[cfg(feature = "tls")]
pub use crate::server::server::tls_config;
