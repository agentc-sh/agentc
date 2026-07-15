// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

pub mod runtime;
pub mod tool;

pub use tool::PythonTool;

#[cfg(feature = "python-embedded")]
pub use runtime::embedded::{EmbeddedRuntime, macros::py_freeze};

#[cfg(feature = "python-static")]
pub use runtime::r#static::{EmbeddedTree, StaticRuntime, embed_dir};
