// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

#[allow(unused_extern_crates)]
extern crate self as agentc_prompt;

pub mod buffer;
pub mod compaction;
pub mod counter;
pub mod env;
pub mod errors;
pub mod template;
pub mod vars;

// Re-export serde_json for use in the context! macro.
#[doc(hidden)]
pub mod __private {
    pub use ::serde_json;
}

pub mod prelude {
    pub use crate::buffer::*;
    pub use crate::compaction::*;
    pub use crate::counter::*;
    pub use crate::env::*;
    pub use crate::errors::*;
    pub use crate::template::*;
    pub use crate::vars::*;
}

pub mod macros {
    pub use crate::context;
}
