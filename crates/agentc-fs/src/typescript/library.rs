// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use agentc_executor_typescript::{
    backend::ExecutorBackend,
    guestjs::{errors::Error, host::library::HostLibrary},
    host::HostRuntime,
};

use crate::{fs::Dir, typescript::module::FsModule};

pub struct FsLibrary;

impl FsLibrary {
    pub fn bind<B>(dir: Dir, runtime: HostRuntime) -> Result<HostLibrary<B>, Error>
    where
        B: ExecutorBackend,
    {
        FsModule::new(dir, runtime).try_into()
    }
}
