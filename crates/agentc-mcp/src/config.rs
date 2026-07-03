// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::{collections::HashMap, time::Duration};

use agentc_agent::types::capability::CapabilitySet;

/// The transport mechanism used to communicate with an MCP server.
#[derive(Debug, Clone)]
pub enum McpTransport {
    /// Spawn a local subprocess and communicate over its stdin/stdout.
    Stdio {
        /// The command to execute, e.g. `./mcp_server`.
        command: String,
        /// Arguments to pass to the command.
        args: Vec<String>,
        /// Environment variables to set for the subprocess.
        env: HashMap<String, String>,
    },
    /// Connect to an MCP server over the streamable HTTP transport.
    StreamableHttp {
        /// The base URL of the MCP server, e.g. `http://localhost:8080`.
        url: String,
        /// An optional auth token to include in the `Authorization` header of each request.
        auth_token: Option<String>,
        /// Additional headers to include in each request.
        headers: HashMap<String, String>,
    },
}

impl McpTransport {
    pub fn stdio(
        command: impl Into<String>,
        args: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self::Stdio {
            command: command.into(),
            args: args
                .into_iter()
                .map(Into::into)
                .collect(),
            env: HashMap::new(),
        }
    }

    pub fn streamable_http(url: impl Into<String>, auth_token: Option<impl Into<String>>) -> Self {
        Self::StreamableHttp {
            url: url.into(),
            auth_token: auth_token.map(Into::into),
            headers: HashMap::new(),
        }
    }
}

/// Controls how [`McpServerHandle`](crate::connection::McpServerHandle) retries
/// after a connection failure.
#[derive(Debug, Clone)]
pub struct ReconnectPolicy {
    /// Maximum number of reconnect attempts before giving up.
    pub max_retries: u32,
    /// Wait time before the first retry attempt.
    pub initial_backoff: Duration,
    /// Upper bound on the wait time between retries.
    pub max_backoff: Duration,
    /// Multiplier applied to the backoff duration after each attempt.
    pub multiplier: f64,
}

impl Default for ReconnectPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_backoff: Duration::from_millis(500),
            max_backoff: Duration::from_secs(30),
            multiplier: 2.0,
        }
    }
}

/// Configuration for a single MCP server connection.
#[derive(Debug, Clone)]
pub struct McpServerConfig {
    /// Logical name identifying this server. Used as the tool name prefix by default.
    pub name: String,
    /// How to establish the transport connection to the server.
    pub transport: McpTransport,
    /// Capabilities assigned to every tool discovered from this server.
    ///
    /// Use this to control which agent contexts are permitted to invoke MCP tools.
    /// For example, a filesystem MCP server might be given `CapabilitySet::from(["mcp::filesystem"])`.
    pub capabilities: CapabilitySet,
    /// Override the prefix prepended to tool names from this server.
    ///
    /// `None` uses the server `name` as the prefix.
    /// `Some("")` disables prefixing entirely; the caller is responsible for
    /// avoiding name collisions with other tools.
    ///
    /// Tool names are formatted as `{prefix}__{tool_name}`.
    pub tool_prefix: Option<String>,
    /// Reconnection behavior on transport failures.
    pub reconnect: ReconnectPolicy,
    /// Timeout for the initial connection and MCP handshake.
    pub connect_timeout: Duration,
    /// Per-call timeout applied to each tool invocation.
    pub tool_call_timeout: Duration,
}

impl McpServerConfig {
    /// Creates a new MCP server configuration with the given name and transport.
    pub fn new(name: impl Into<String>, transport: McpTransport) -> Self {
        Self {
            name: name.into(),
            transport,
            capabilities: CapabilitySet::empty(),
            tool_prefix: None,
            reconnect: ReconnectPolicy::default(),
            connect_timeout: Duration::from_secs(10),
            tool_call_timeout: Duration::from_secs(60),
        }
    }

    /// Changes the capabilities assigned to tools from this server.
    pub fn with_capabilities(mut self, capabilities: impl Into<CapabilitySet>) -> Self {
        self.capabilities = capabilities.into();
        self
    }

    /// Changes the tool name prefix for this server.
    pub fn with_tool_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.tool_prefix = Some(prefix.into());
        self
    }

    /// Changes the reconnection policy for this server.
    pub fn with_reconnect_policy(mut self, policy: ReconnectPolicy) -> Self {
        self.reconnect = policy;
        self
    }

    /// Changes the connection timeout for this server.
    pub fn with_connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }

    pub fn with_tool_call_timeout(mut self, timeout: Duration) -> Self {
        self.tool_call_timeout = timeout;
        self
    }

    /// Returns the effective tool name prefix for this server.
    ///
    /// Falls back to [`McpServerConfig::name`](crate::config::McpServerConfig::name) if no explicit prefix is configured.
    pub fn effective_prefix(&self) -> &str {
        self.tool_prefix
            .as_deref()
            .unwrap_or(&self.name)
    }

    /// Returns the prefixed tool name for an MCP tool with the given original name.
    ///
    /// If the effective prefix is empty the original name is returned unchanged.
    pub fn prefixed_name(&self, mcp_name: &str) -> String {
        let prefix = self.effective_prefix();

        if prefix.is_empty() {
            mcp_name.to_string()
        } else {
            format!("{}__{}", prefix, mcp_name)
        }
    }
}
