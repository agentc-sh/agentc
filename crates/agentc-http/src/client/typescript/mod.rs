// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

pub mod fetch;
pub mod headers;
pub mod module;
pub mod response;

mod errors;

use agentc_executor_typescript::guestjs::host::HostLibrary;

use crate::client::{builder::HttpClientBuilder, client::HttpClient, typescript::module::HttpModule};

/// The guest capabilities exposed by this crate.
pub struct HttpLibrary;

impl HttpLibrary {
    /// Binds an existing client.
    pub fn bind(client: impl Into<HttpClient>) -> HostLibrary {
        HostLibrary::new().with(HttpModule::new(client))
    }

    /// Binds one client per guest, built from a shared configuration.
    pub fn bind_guest(builder: impl Into<HttpClientBuilder>) -> HostLibrary {
        HostLibrary::new().with(HttpModule::per_guest(builder))
    }
}
