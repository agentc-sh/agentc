// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

mod expression;

pub mod file;
pub mod runtime;

pub use file::{FileFunctionDeserialize, FileReader, RootedFileReader};
pub use runtime::RuntimeFunctionDeserialize;
