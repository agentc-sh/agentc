// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use agentc_executor_python::{
    backend::ExecutorBackend,
    guestpy::{errors::Error, host::library::HostLibrary},
};

use crate::python::bindings::module::ToolsModule;

pub struct ToolsLibrary;

impl ToolsLibrary {
    pub fn bind<B>() -> Result<HostLibrary<B>, Error>
    where
        B: ExecutorBackend,
    {
        Ok(HostLibrary::new().with(ToolsModule::new().try_into()?))
    }
}
