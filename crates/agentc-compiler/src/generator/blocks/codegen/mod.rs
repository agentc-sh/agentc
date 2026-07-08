// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

pub mod block;
pub mod ident;
pub mod traits;

pub use block::CodeGenBlock;
pub use ident::ToIdent;
pub use traits::CodeGen;
