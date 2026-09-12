// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::future::Future;

use agentc_executor_python::guestpy::{errors::Error, host_class};

use crate::python::bindings::output::ToolOutput;

pub struct Tool;

#[host_class(backend = B, generic, crate_path = agentc_executor_python::guestpy)]
impl Tool {
    #[guestpy(constructor)]
    fn new() -> Result<Self, Error> {
        Ok(Self)
    }

    #[guestpy(async_method)]
    fn execute(
        &self,
    ) -> Result<impl Future<Output = Result<ToolOutput<B>, Error>> + use<B>, Error> {
        Ok(async { Err(Error::unsupported("a Tool subclass must implement execute")) })
    }
}
