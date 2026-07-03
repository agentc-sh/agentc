// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::{
    collections::{HashMap, hash_map::Values},
    fmt::{Debug, Formatter, Result as FmtResult},
    sync::Arc,
};

use crate::{
    graph::state::GraphState,
    tools::traits::{ErasedTool, ErasedToolWrapper, Tool, TypedTool, TypedToolWrapper},
};

#[derive(Clone)]
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn ErasedTool>>,
}

impl ToolRegistry {
    pub fn new(tools: HashMap<String, Arc<dyn ErasedTool>>) -> Self {
        Self { tools }
    }

    pub fn empty() -> Self {
        Self { tools: HashMap::new() }
    }

    pub fn builder() -> ToolRegistryBuilder {
        ToolRegistryBuilder::new()
    }

    pub fn register<S, T>(&mut self, tool: T)
    where
        T: Tool<S> + 'static,
        S: GraphState + 'static,
    {
        self.tools
            .insert(tool.definition().name, Arc::new(ErasedToolWrapper::new(tool)));
    }

    pub fn register_typed<S, T>(&mut self, tool: T)
    where
        T: TypedTool<S> + 'static,
        S: GraphState + 'static,
    {
        self.register(TypedToolWrapper::new(tool));
    }

    pub fn merge(&mut self, other: Self) {
        self.tools.extend(other.tools);
    }

    pub fn merged_with(mut self, other: Self) -> Self {
        self.merge(other);
        self
    }

    pub fn into_tools(self) -> HashMap<String, Arc<dyn ErasedTool>> {
        self.tools
    }

    pub fn tools(&self) -> Values<'_, String, Arc<dyn ErasedTool>> {
        self.tools.values()
    }

    pub fn get(&self, name: impl AsRef<str>) -> Option<Arc<dyn ErasedTool>> {
        self.tools.get(name.as_ref()).cloned()
    }
}

pub struct ToolRegistryBuilder {
    tools: HashMap<String, Arc<dyn ErasedTool>>,
}

impl Default for ToolRegistryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolRegistryBuilder {
    pub fn new() -> Self {
        Self { tools: HashMap::new() }
    }

    pub fn with_tool<S, T>(mut self, tool: T) -> Self
    where
        T: Tool<S> + 'static,
        S: GraphState + 'static,
    {
        self.tools
            .insert(tool.definition().name, Arc::new(ErasedToolWrapper::new(tool)));
        self
    }

    pub fn with_typed_tool<S, T>(self, tool: T) -> Self
    where
        T: TypedTool<S> + 'static,
        S: GraphState + 'static,
    {
        self.with_tool(TypedToolWrapper::new(tool))
    }

    pub fn with_tool_boxed(mut self, tool: Arc<dyn ErasedTool>) -> Self {
        self.tools
            .insert(tool.definition().name, tool);
        self
    }

    pub fn build(self) -> ToolRegistry {
        ToolRegistry::new(self.tools)
    }
}

impl Debug for ToolRegistry {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.debug_struct("ToolRegistry")
            .field("tools", &self.tools.keys().collect::<Vec<_>>())
            .finish()
    }
}
