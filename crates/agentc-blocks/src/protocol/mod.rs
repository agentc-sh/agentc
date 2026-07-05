// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

pub mod resolver;
pub mod traits;
pub mod types;

pub use resolver::{ProtocolResolver, ProtocolResolverBuilder};
pub use traits::{ErasedProtocol, Protocol};
pub use types::ResolvedProtocol;
