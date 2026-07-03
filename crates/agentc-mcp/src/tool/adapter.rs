// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use serde_json::Value;
use std::sync::Arc;

use agentc_agent::{
    graph::state::GraphState,
    tools::{
        errors::ToolError,
        traits::Tool,
        types::{ToolInput, ToolOutput},
    },
    types::{capability::CapabilitySet, tools::ToolDefinition},
};
use async_trait::async_trait;

use crate::connection::McpServerHandle;

/// An [`agentc_agent::tools::traits::Tool`] adapter that wraps a single tool
/// exposed by an MCP server.
///
/// `McpToolAdapter` is constructed for each tool discovered during
/// [`McpServerHandle::connect`](crate::connection::McpServerHandle::connect).
/// Multiple adapters for the same server share one [`McpServerHandle`] via an
/// [`Arc`], so reconnection logic is centralised there.
///
/// Because MCP tools return plain JSON and have no knowledge of graph state,
/// `StateUpdate` is always `()`.
pub struct McpToolAdapter {
    handle: Arc<McpServerHandle>,
    mcp_name: String,
    definition: ToolDefinition,
    capabilities: CapabilitySet,
}

impl McpToolAdapter {
    /// Construct an adapter from a server handle and a single MCP tool definition.
    ///
    /// `prefix` is prepended to the tool name as `{prefix}__{tool_name}`. Pass an
    /// empty string to disable prefixing.
    pub fn new(handle: Arc<McpServerHandle>, mcp_tool: &rmcp::model::Tool) -> Self {
        let handle_config = handle.config();

        Self {
            mcp_name: mcp_tool.name.to_string(),
            definition: ToolDefinition {
                name: handle_config.prefixed_name(&mcp_tool.name),
                description: mcp_tool
                    .description
                    .as_deref()
                    .unwrap_or("")
                    .to_string(),
                parameters: Value::Object(mcp_tool.input_schema.as_ref().clone()),
            },
            capabilities: handle_config.capabilities.clone(),
            handle,
        }
    }
}

#[async_trait]
impl<S: GraphState + 'static> Tool<S> for McpToolAdapter {
    type State = ();
    type StateUpdate = ();

    fn definition(&self) -> ToolDefinition {
        self.definition.clone()
    }

    fn capabilities(&self) -> CapabilitySet {
        self.capabilities.clone()
    }

    async fn execute(&self, input: ToolInput) -> Result<ToolOutput<()>, ToolError> {
        self.handle
            .call_tool(&self.mcp_name, input.args)
            .await
            .map(ToolOutput::ok)
            .map_err(|e| ToolError::execution_error(&self.definition.name, e.to_string()))
    }
}
