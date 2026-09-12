// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use agentc_executor_typescript::{guestjs::host::HostLibrary, host::HostRuntime};

use crate::{fs::Dir, typescript::module::FsModule};

pub struct FsLibrary;

impl FsLibrary {
    pub fn bind(dir: Dir, host_runtime: HostRuntime) -> HostLibrary {
        HostLibrary::new().with(FsModule::new(dir, host_runtime))
    }
}
