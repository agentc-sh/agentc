// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

#[allow(unused_extern_crates)]
extern crate self as agentc_executor_python;

pub use guestpy;

#[macro_export]
macro_rules! bundle {
    ($path:literal) => {
        $crate::guestpy::bundle!($path, crate_path = $crate::guestpy)
            .map_err(agentc_executor_python::errors::Error::from)
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

mod job;
mod worker;
