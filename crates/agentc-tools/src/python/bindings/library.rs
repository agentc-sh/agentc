// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use agentc_executor_python::guestpy::{
    backend::{
        Backend, BackendCallables, BackendClasses, BackendCoroutines, BackendExceptions,
        BackendModules, BackendValues,
    },
    host::library::HostLibrary,
};

use crate::python::bindings::module::ToolsModule;

pub struct ToolsLibrary;

impl ToolsLibrary {
    pub fn bind<B>() -> HostLibrary<B>
    where
        B: Backend
            + BackendValues
            + BackendCallables
            + BackendClasses
            + BackendModules
            + BackendCoroutines
            + BackendExceptions,
    {
        HostLibrary::new().with(ToolsModule::new().into())
    }
}
