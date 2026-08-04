// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

#[allow(unused_extern_crates)]
extern crate self as agentc_core;

pub mod build;
pub mod errors;
pub mod generate;
pub mod init;
pub mod inspect;
pub mod manifest;
pub mod parser;
pub mod pipeline;
pub mod run;

pub mod blocks {
    pub use agentc_blocks::*;
}

pub mod compiler {
    pub use agentc_compiler::*;
}
