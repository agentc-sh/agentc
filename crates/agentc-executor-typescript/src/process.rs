// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use guestjs::{
    eval::GuestGlobalsExt,
    handle::{object::Object, protocol::ObjectProtocol},
    host::library::{HostInitializer, HostLibrary},
};

use crate::backend::ExecutorBackend;

pub struct ProcessEnvironment;

impl ProcessEnvironment {
    pub fn lib<B: ExecutorBackend>() -> Result<HostLibrary<B>, guestjs::errors::Error> {
        Ok(
            HostLibrary::from(HostInitializer::new(|enter| {
                let globals = enter.guest().globals()?;
                let object = globals.class("Object")?;
                let env = object.construct(())?;

                for (name, value) in std::env::vars() {
                    env.set(name, value)?;
                }

                let process = match globals.get::<Option<Object<B>>, _>("process")? {
                    Some(process) => process,
                    None => object.construct_as::<_, Object<B>>(())?,
                };

                process.set("env", env)?;
                globals.set("process", process)
            }))
        )
    }
}
