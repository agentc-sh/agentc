// SPDX-FileCopyrightText: 2026 Timothy Pogue
//
// SPDX-License-Identifier: LicenseRef-Proprietary

mod context;
mod embed;
mod errors;
mod interpreter;
mod runtime;

pub use embed::EmbeddedTree;
pub use runtime::StaticRuntime;

pub mod macros {
    pub use include_dir::include_dir as embed_dir;
}
