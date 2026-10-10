// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

mod builder;
mod dir;
mod file;
mod filesystem;
mod namespace;
mod types;

pub use builder::FsBuilder;
pub use dir::{Dir, DirEntries, DirEntry, Walk};
pub use file::File;
pub use filesystem::Fs;
pub use types::*;
