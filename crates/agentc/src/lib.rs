// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

#[allow(unused_extern_crates)]
extern crate self as agentc;

pub mod core {
    pub use agentc_core::*;
}

#[cfg(feature = "cli")]
pub mod cli;
