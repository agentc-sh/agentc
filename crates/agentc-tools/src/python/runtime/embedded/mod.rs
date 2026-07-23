// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

mod runtime;

pub use runtime::EmbeddedRuntime;

#[doc(hidden)]
pub use rustpython_vm as __rustpython_vm;

pub mod macros {
    #[macro_export]
    macro_rules! py_freeze {
        ($($args:tt)*) => {{
            use $crate::python::runtime::embedded::__rustpython_vm as rustpython_vm;

            $crate::python::runtime::embedded::__rustpython_vm::py_freeze!(
                $($args)*,
                crate_name = "rustpython_vm"
            )
        }};
    }
}

pub use crate::py_freeze;
