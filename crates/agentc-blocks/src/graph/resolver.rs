// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use serde_json::Value;
use std::collections::HashMap;

use crate::{
    context::ResolvedContext,
    errors::BlocksError,
    graph::{
        traits::{AgentGraph, ErasedAgentGraph},
        types::ResolvedGraph,
    },
};

pub struct GraphResolver {
    graphs: HashMap<String, Box<dyn ErasedAgentGraph>>,
}

impl GraphResolver {
    pub fn new(graphs: HashMap<String, Box<dyn ErasedAgentGraph>>) -> Self {
        Self { graphs }
    }

    pub fn builder() -> GraphResolverBuilder {
        GraphResolverBuilder::default()
    }

    pub fn register<T>(&mut self, graph: T) -> Result<(), BlocksError>
    where
        T: AgentGraph + 'static,
    {
        let name = graph.name().to_string();

        if self.graphs.contains_key(&name) {
            return Err(BlocksError::duplicate_registration("graph", name));
        }

        self.graphs
            .insert(name, Box::new(graph));

        Ok(())
    }

    pub fn resolve(
        &self,
        graph_name: &str,
        context: ResolvedContext,
        config: Value,
    ) -> Result<ResolvedGraph, BlocksError> {
        self.graphs
            .get(graph_name)
            .ok_or_else(|| BlocksError::UnknownGraph(graph_name.to_string()))?
            .resolve_erased(context, config)
    }
}

pub struct GraphResolverBuilder {
    error: Option<BlocksError>,
    graphs: HashMap<String, Box<dyn ErasedAgentGraph>>,
}

impl GraphResolverBuilder {
    pub fn new() -> Self {
        Self {
            error: None,
            graphs: HashMap::new(),
        }
    }

    pub fn with_graph<T>(mut self, graph: T) -> Self
    where
        T: AgentGraph + 'static,
    {
        let name = graph.name().to_string();

        if self.graphs.contains_key(&name) {
            self.error = Some(BlocksError::duplicate_registration("graph", name));
            return self;
        }

        self.graphs
            .insert(name, Box::new(graph));

        self
    }

    pub fn build(self) -> Result<GraphResolver, BlocksError> {
        if let Some(error) = self.error {
            return Err(error);
        }

        Ok(GraphResolver::new(self.graphs))
    }
}

impl Default for GraphResolverBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};
    use serde_json::json;

    use super::*;
    use crate::composition::GenerationContribution;

    #[derive(Debug, Clone, Deserialize, Serialize)]
    struct TestGraphConfig {
        enabled: bool,
    }

    struct TestGraph;

    impl AgentGraph for TestGraph {
        type Config = TestGraphConfig;

        fn name(&self) -> &str {
            "test"
        }

        fn resolve(
            &self,
            _context: ResolvedContext,
            config: Self::Config,
        ) -> Result<ResolvedGraph, BlocksError> {
            if !config.enabled {
                return Err(BlocksError::invalid("graph must be enabled"));
            }

            Ok(ResolvedGraph {
                name: AgentGraph::name(self).to_string(),
                contribution: GenerationContribution::new(),
                integrations: Vec::new(),
            })
        }
    }

    fn context() -> ResolvedContext {
        serde_json::from_value(json!({
            "slug": "assistant",
            "agent_name": "assistant",
            "runtime": {
                "default_tenant_id": "default"
            },
            "providers": [],
            "agent": {
                "version": "0.1.0",
                "description": null,
                "prompt": null,
                "capabilities": null,
                "capability_policy": null,
                "model": {
                    "provider": "anthropic",
                    "name": "claude"
                }
            },
            "blocks": {},
            "tools": {},
            "skills": {},
            "http_server": null
        }))
        .unwrap()
    }

    #[test]
    fn builder_rejects_duplicate_graph_registration() {
        let result = GraphResolver::builder()
            .with_graph(TestGraph)
            .with_graph(TestGraph)
            .build();

        assert!(matches!(
            result,
            Err(BlocksError::DuplicateRegistration {
                component: "graph",
                ..
            })
        ));
    }

    #[test]
    fn resolver_rejects_unknown_graph() {
        let resolver = GraphResolver::builder()
            .with_graph(TestGraph)
            .build()
            .unwrap();

        let result = resolver
            .resolve("missing", context(), json!({}));

        assert!(matches!(result, Err(BlocksError::UnknownGraph(_))));
    }

    #[test]
    fn resolver_dispatches_typed_config() {
        let resolver = GraphResolver::builder()
            .with_graph(TestGraph)
            .build()
            .unwrap();
        let graph = resolver
            .resolve("test", context(), json!({ "enabled": true }))
            .unwrap();

        assert_eq!(graph.name, "test");
    }
}
