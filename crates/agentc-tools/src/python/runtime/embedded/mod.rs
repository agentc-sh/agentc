// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

mod runtime;

pub use runtime::EmbeddedRuntime;

pub mod macros {
    pub use rustpython_vm::py_freeze;
}
