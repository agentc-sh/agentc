// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use agentc_executor_typescript::{
    backend::ExecutorBackend,
    guestjs::{
        errors::Error,
        host::{library::HostLibrary, module::ModuleDeclaration},
    },
};

use crate::javascript::bindings::module::ToolsModule;

pub struct ToolsLibrary;

impl ToolsLibrary {
    pub fn bind<B>() -> Result<HostLibrary<B>, Error>
    where
        B: ExecutorBackend,
    {
        Ok(
            HostLibrary::new()
                .extend(ModuleDeclaration::<B>::try_from(ToolsModule::new())?)
        )
    }
}
