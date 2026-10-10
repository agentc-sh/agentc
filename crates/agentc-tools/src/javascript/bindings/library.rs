// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use agentc_executor_typescript::guestjs::host::HostLibrary;

use crate::javascript::bindings::module::ToolsModule;

pub struct ToolsLibrary;

impl ToolsLibrary {
    pub fn bind() -> HostLibrary {
        HostLibrary::new().with(ToolsModule::new())
    }
}
