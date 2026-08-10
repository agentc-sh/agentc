// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

mod context;
mod filesystem;
mod traits;

pub use context::{
    AccessContext, EntriesContext, MetadataContext, OpenContext, RemoveContext, RenameContext,
    SymlinkContext, WriteContext,
};
pub use filesystem::PolicyFs;
pub use traits::{Denied, Policy};
