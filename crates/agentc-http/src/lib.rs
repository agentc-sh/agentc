// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

#[allow(unused_extern_crates)]
extern crate self as agentc_http;

#[cfg(feature = "client")]
pub mod client;

#[cfg(feature = "server")]
pub mod server;
