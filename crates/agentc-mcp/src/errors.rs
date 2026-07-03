// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::time::Duration;
use thiserror::Error;

/// Errors produced by the MCP integration layer.
#[derive(Debug, Error)]
pub enum McpError {
    #[error("failed to connect to MCP server '{name}': {source}")]
    ConnectionFailed {
        name: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("MCP server '{name}' is unavailable after {retries} retries")]
    ServerUnavailable { name: String, retries: u32 },

    #[error("tool call '{tool}' on server '{server}' timed out after {timeout:?}")]
    ToolCallTimeout {
        server: String,
        tool: String,
        timeout: Duration,
    },

    #[error("MCP server '{server}' returned an error for tool '{tool}': {message}")]
    ToolExecutionFailed {
        server: String,
        tool: String,
        message: String,
    },

    #[error("MCP protocol error on server '{server}': {message}")]
    Protocol { server: String, message: String },

    #[error("transport error on server '{server}': {source}")]
    Transport {
        server: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

impl McpError {
    /// Creates a new [`McpError::ConnectionFailed`](crate::errors::McpError::ConnectionFailed) with the given server name and source error.
    pub fn connection_failed(
        name: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::ConnectionFailed {
            name: name.into(),
            source: Box::new(source),
        }
    }

    /// Creates a new [`McpError::ServerUnavailable`](crate::errors::McpError::ServerUnavailable) with the given server name and retry count.
    pub fn unavailable(name: impl Into<String>, retries: u32) -> Self {
        Self::ServerUnavailable { name: name.into(), retries }
    }

    /// Creates a new [`McpError::ToolCallTimeout`](crate::errors::McpError::ToolCallTimeout) with the given server name, tool name, and timeout duration.
    pub fn timed_out(
        server: impl Into<String>,
        tool: impl Into<String>,
        timeout: Duration,
    ) -> Self {
        Self::ToolCallTimeout {
            server: server.into(),
            tool: tool.into(),
            timeout,
        }
    }

    /// Creates a new [`McpError::ToolExecutionFailed`](crate::errors::McpError::ToolExecutionFailed) with the given server name, tool name, and error message.
    pub fn execution_failed(
        server: impl Into<String>,
        tool: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::ToolExecutionFailed {
            server: server.into(),
            tool: tool.into(),
            message: message.into(),
        }
    }

    /// Creates a new [`McpError::Protocol`](crate::errors::McpError::Protocol) with the given server name and error message.
    pub fn transport(
        server: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::Transport {
            server: server.into(),
            source: Box::new(source),
        }
    }

    /// Creates a new [`McpError::Protocol`](crate::errors::McpError::Protocol) with the given server name and error message.
    pub fn protocol(server: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Protocol {
            server: server.into(),
            message: message.into(),
        }
    }

    /// Returns `true` if this error indicates a connection or transport failure
    /// that may be recoverable by reconnecting.
    pub fn is_connection_error(&self) -> bool {
        matches!(self, Self::Transport { .. } | Self::Protocol { .. })
    }
}
