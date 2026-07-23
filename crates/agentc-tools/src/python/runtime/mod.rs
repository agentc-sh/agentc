// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

mod errors;
mod protocol;
mod traits;

#[cfg(feature = "python-embedded")]
pub mod embedded;

#[cfg(feature = "python-static")]
pub mod r#static;

pub use errors::RuntimeError;
pub use protocol::{ArgValue, FunctionArgs, NativeCallable, PyFuture};
pub use traits::{Runtime, RuntimeExt};
