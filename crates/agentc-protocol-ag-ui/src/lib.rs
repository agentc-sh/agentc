// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

#[allow(unused_extern_crates)]
extern crate self as agentc_protocol_ag_ui;

pub mod protocol;
pub mod router;
pub mod traits;

pub mod prelude {
    pub use crate::{protocol::*, router::*, traits::*};
}
