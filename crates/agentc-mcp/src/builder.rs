// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::sync::Arc;

use agentc_agent::{
    agent::AgentBuilder,
    context::AgentContext,
    graph::state::{GraphNode, GraphState, StateOf},
    tools::{registry::ToolRegistryBuilder, traits::ErasedToolWrapper},
    types::event::AgentEvent,
};
use async_trait::async_trait;

use crate::registry::McpRegistry;

/// Extension methods on [`ToolRegistryBuilder`](agentc_agent::tools::registry::ToolRegistryBuilder)
/// for registering MCP server tools.
#[async_trait]
pub trait ToolRegistryBuilderMcpExt: Sized + Send {
    /// Register all tools from a pre-built [`McpRegistry`] into this builder.
    ///
    /// Each discovered MCP tool becomes one entry in the
    /// [`ToolRegistry`](agentc_agent::tools::registry::ToolRegistry), keyed by
    /// its prefixed name (e.g. `filesystem/read_file`).
    ///
    /// Provide the concrete graph state type `S` for the agent being built.
    async fn with_mcp_registry<S: GraphState + 'static>(self, registry: &McpRegistry) -> Self;
}

#[async_trait]
impl ToolRegistryBuilderMcpExt for ToolRegistryBuilder {
    async fn with_mcp_registry<S: GraphState + 'static>(mut self, registry: &McpRegistry) -> Self {
        for adapter in registry.tool_adapters().await {
            self = self.with_tool_boxed(Arc::new(ErasedToolWrapper::<_, S>::new(adapter)));
        }
        self
    }
}

/// Extension methods on [`AgentBuilder`](agentc_agent::agent::AgentBuilder) for registering MCP server tools.
#[async_trait]
pub trait AgentBuilderMcpExt: Sized + Send {
    /// Register all tools from a pre-built [`McpRegistry`] into this builder.
    ///
    /// Each discovered MCP tool becomes one entry in the
    /// [`ToolRegistry`](agentc_agent::tools::registry::ToolRegistry), keyed by
    /// its prefixed name (e.g. `filesystem/read_file`).
    ///
    /// Provide the concrete graph state update type `U` for the agent being built.
    async fn with_mcp_registry(self, registry: &McpRegistry) -> Self;
}

#[async_trait]
impl<N, E, M> AgentBuilderMcpExt for AgentBuilder<N, E, M>
where
    N: GraphNode<Context = AgentContext<E, M>> + 'static,
    E: From<AgentEvent<StateOf<N>>> + Send + Clone + 'static,
    M: Send + Clone + 'static,
{
    async fn with_mcp_registry(self, registry: &McpRegistry) -> Self {
        self.with_tool_registry(
            ToolRegistryBuilder::new()
                .with_mcp_registry::<StateOf<N>>(registry)
                .await
                .build(),
        )
    }
}
