// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use agentc_executor_python::{
    backend::ExecutorBackend,
    guestpy::{errors::Error, host::library::HostLibrary},
};

use crate::client::{client::HttpClient, python::module::HttpModule};

pub struct HttpLibrary;

impl HttpLibrary {
    pub fn bind<B>(client: impl Into<HttpClient>) -> Result<HostLibrary<B>, Error>
    where
        B: ExecutorBackend,
    {
        Ok(HostLibrary::new().with(HttpModule::new(client).try_into()?))
    }
}
