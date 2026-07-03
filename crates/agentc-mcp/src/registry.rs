// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::{collections::HashMap, sync::Arc};

use futures::future::try_join_all;

use crate::{
    config::McpServerConfig, connection::McpServerHandle, errors::McpError, tool::McpToolAdapter,
};

/// A collection of connected MCP server handles.
///
/// Built via [`McpRegistryBuilder`](crate::registry::McpRegistryBuilder) by calling
/// [`McpRegistryBuilder::build`](crate::registry::McpRegistryBuilder::build),
/// which connects all configured servers concurrently.
pub struct McpRegistry {
    servers: HashMap<String, Arc<McpServerHandle>>,
}

impl McpRegistry {
    /// Returns a builder for constructing an [`McpRegistry`].
    pub fn builder() -> McpRegistryBuilder {
        McpRegistryBuilder::default()
    }

    /// Returns the handle for the server with the given logical name, if present.
    pub fn server(&self, name: &str) -> Option<&Arc<McpServerHandle>> {
        self.servers.get(name)
    }

    /// Returns an iterator over the logical names of all registered servers.
    pub fn server_names(&self) -> impl Iterator<Item = &str> {
        self.servers.keys().map(String::as_str)
    }

    /// Returns one [`McpToolAdapter`] per discovered MCP tool across all servers.
    ///
    /// The adapters are ready to be inserted into a
    /// [`ToolRegistryBuilder`](agentc_agent::tools::registry::ToolRegistryBuilder)
    /// via [`ToolRegistryBuilderMcpExt`](crate::builder::ToolRegistryBuilderMcpExt).
    pub async fn tool_adapters(&self) -> Vec<McpToolAdapter> {
        let mut adapters = Vec::new();

        for handle in self.servers.values() {
            for tool in handle.cached_tools().await {
                adapters.push(McpToolAdapter::new(Arc::clone(handle), &tool));
            }
        }

        adapters
    }
}

/// Builder for [`McpRegistry`](crate::registry::McpRegistry).
///
/// Add server configurations with [`McpRegistryBuilder::with_server`](crate::registry::McpRegistryBuilder::with_server), then call
/// [`McpRegistryBuilder::build`](crate::registry::McpRegistryBuilder::build) to connect all servers concurrently.
#[derive(Default)]
pub struct McpRegistryBuilder {
    configs: Vec<McpServerConfig>,
}

impl McpRegistryBuilder {
    /// Add an MCP server to be connected when [`McpRegistryBuilder::build`](crate::registry::McpRegistryBuilder::build) is called.
    pub fn with_server(mut self, config: McpServerConfig) -> Self {
        self.configs.push(config);
        self
    }

    /// Connect all configured servers concurrently.
    ///
    /// Returns an error if any server fails to connect within its
    /// [`McpServerConfig::connect_timeout`](crate::config::McpServerConfig::connect_timeout).
    pub async fn build(self) -> Result<McpRegistry, McpError> {
        Ok(McpRegistry {
            servers: try_join_all(
                self.configs
                    .into_iter()
                    .map(McpServerHandle::connect),
            )
            .await?
            .into_iter()
            .map(|handle| (handle.config().name.clone(), handle))
            .collect(),
        })
    }
}
