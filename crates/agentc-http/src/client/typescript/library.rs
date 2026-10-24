// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use agentc_executor_typescript::{
    backend::ExecutorBackend,
    guestjs::{errors::Error, host::library::HostLibrary},
};

use crate::client::{client::HttpClient, typescript::module::HttpModule};

/// The guest capabilities exposed by this crate.
pub struct HttpLibrary;

impl HttpLibrary {
    /// Binds an existing client.
    pub fn bind<B>(client: impl Into<HttpClient>) -> Result<HostLibrary<B>, Error>
    where
        B: ExecutorBackend,
    {
        Ok(HttpModule::new(client).try_into()?)
    }
}
