// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

#[allow(unused_extern_crates)]
extern crate self as agentc_protocol_a2a;

pub mod protocol;

#[cfg(feature = "client")]
pub mod client;

#[cfg(feature = "server")]
pub mod traits;

#[cfg(feature = "server")]
pub mod router;

pub mod prelude {
    pub use crate::protocol::*;

    #[cfg(feature = "client")]
    pub use crate::client::*;

    #[cfg(feature = "server")]
    pub use crate::{router::*, traits::*};
}
