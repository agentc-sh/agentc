// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

#[allow(unused_extern_crates)]
extern crate self as agentc_fs;

pub mod errors;
pub mod fs;
pub mod path;

pub mod backend;

#[cfg(feature = "embedded")]
pub mod embedded;

#[cfg(feature = "typescript")]
pub mod typescript;

pub mod host;
pub mod memory;
pub mod mount;
pub mod overlay;
pub mod policy;
pub mod readonly;

pub use crate::{errors::Error, fs::Fs};
