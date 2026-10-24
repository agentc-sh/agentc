// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

#[allow(unused_extern_crates)]
extern crate self as agentc_executor_typescript;

pub use guestjs;

#[macro_export]
macro_rules! bundle {
    ($path:literal) => {
        $crate::guestjs::bundle!($path, crate_path = $crate::guestjs)
            .map_err(agentc_executor_typescript::errors::Error::from)
    };
}

pub mod backend;
pub mod context;
pub mod errors;
pub mod execution;
pub mod executor;
pub mod host;
pub mod json;
pub mod lease;
pub mod process;

mod job;
mod worker;
