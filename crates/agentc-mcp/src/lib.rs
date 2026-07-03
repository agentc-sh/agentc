// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

#[allow(unused_extern_crates)]
extern crate self as agentc_mcp;

pub mod builder;
pub mod config;
pub mod connection;
pub mod errors;
pub mod registry;
pub mod tool;

pub mod prelude {
    pub use crate::builder::{AgentBuilderMcpExt, ToolRegistryBuilderMcpExt};
    pub use crate::config::{McpServerConfig, McpTransport, ReconnectPolicy};
    pub use crate::connection::McpServerHandle;
    pub use crate::errors::McpError;
    pub use crate::registry::{McpRegistry, McpRegistryBuilder};
    pub use crate::tool::McpToolAdapter;
}
