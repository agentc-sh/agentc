// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use crate::{
    archetype::{resolver::ArchetypeResolver, traits::Archetype},
    errors::BlocksError,
    graph::{AgentGraph, GraphResolver, GraphResolverBuilder},
    protocol::{Protocol, ProtocolResolver, ProtocolResolverBuilder},
};

pub struct CompilationCatalog {
    archetypes: ArchetypeResolver,
    graphs: GraphResolver,
    protocols: ProtocolResolver,
}

impl CompilationCatalog {
    pub fn builder() -> CompilationCatalogBuilder {
        CompilationCatalogBuilder::new()
    }

    pub fn new(
        archetypes: ArchetypeResolver,
        graphs: GraphResolver,
        protocols: ProtocolResolver,
    ) -> Self {
        Self {
            archetypes,
            graphs,
            protocols,
        }
    }

    pub fn archetypes(&self) -> &ArchetypeResolver {
        &self.archetypes
    }

    pub fn graphs(&self) -> &GraphResolver {
        &self.graphs
    }

    pub fn protocols(&self) -> &ProtocolResolver {
        &self.protocols
    }
}

pub struct CompilationCatalogBuilder {
    archetypes: Option<ArchetypeResolver>,
    archetype_builder: crate::archetype::resolver::ArchetypeResolverBuilder,
    graphs: Option<GraphResolver>,
    graph_builder: GraphResolverBuilder,
    protocols: Option<ProtocolResolver>,
    protocol_builder: ProtocolResolverBuilder,
    error: Option<BlocksError>,
}

impl CompilationCatalogBuilder {
    pub fn new() -> Self {
        Self {
            archetypes: None,
            archetype_builder: ArchetypeResolver::builder(),
            graphs: None,
            graph_builder: GraphResolver::builder(),
            protocols: None,
            protocol_builder: ProtocolResolver::builder(),
            error: None,
        }
    }

    pub fn with_archetype<T>(mut self, archetype: T) -> Self
    where
        T: Archetype + 'static,
    {
        if let Some(resolver) = &mut self.archetypes {
            if let Err(error) = resolver.register(archetype) {
                self.error = Some(error);
            }

            return self;
        }

        self.archetype_builder = self.archetype_builder.with_archetype(archetype);
        self
    }

    pub fn with_graph<T>(mut self, graph: T) -> Self
    where
        T: AgentGraph + 'static,
    {
        if let Some(resolver) = &mut self.graphs {
            if let Err(error) = resolver.register(graph) {
                self.error = Some(error);
            }

            return self;
        }

        self.graph_builder = self.graph_builder.with_graph(graph);
        self
    }

    pub fn with_protocol<T>(mut self, protocol: T) -> Self
    where
        T: Protocol + 'static,
    {
        if let Some(resolver) = &mut self.protocols {
            if let Err(error) = resolver.register(protocol) {
                self.error = Some(error);
            }

            return self;
        }

        self.protocol_builder = self.protocol_builder.with_protocol(protocol);
        self
    }

    pub fn archetype_resolver(mut self, resolver: ArchetypeResolver) -> Self {
        self.archetypes = Some(resolver);
        self
    }

    pub fn graph_resolver(mut self, resolver: GraphResolver) -> Self {
        self.graphs = Some(resolver);
        self
    }

    pub fn protocol_resolver(mut self, resolver: ProtocolResolver) -> Self {
        self.protocols = Some(resolver);
        self
    }

    pub fn build(self) -> Result<CompilationCatalog, BlocksError> {
        if let Some(error) = self.error {
            return Err(error);
        }

        Ok(CompilationCatalog::new(
            match self.archetypes {
                Some(resolver) => resolver,
                None => self.archetype_builder.build()?,
            },
            match self.graphs {
                Some(resolver) => resolver,
                None => self.graph_builder.build()?,
            },
            match self.protocols {
                Some(resolver) => resolver,
                None => self.protocol_builder.build()?,
            },
        ))
    }
}

impl Default for CompilationCatalogBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};
    use serde_json::json;

    use super::*;
    use crate::{
        archetype::{traits::Archetype, types::ResolvedArchetype},
        composition::GenerationContribution,
        context::ResolvedContext,
        graph::types::ResolvedGraph,
        protocol::types::ResolvedProtocol,
    };

    #[derive(Debug, Clone, Deserialize, Serialize, Default)]
    struct TestArchetypeConfig {}

    struct TestArchetype;

    impl Archetype for TestArchetype {
        type Config = TestArchetypeConfig;

        fn name(&self) -> &str {
            "test"
        }

        fn resolve(
            &self,
            _context: ResolvedContext,
            _config: Self::Config,
        ) -> Result<ResolvedArchetype, BlocksError> {
            Err(BlocksError::unexpected("not needed for catalog tests"))
        }
    }

    #[derive(Debug, Clone, Deserialize, Serialize, Default)]
    struct TestGraphConfig {}

    struct TestGraph;

    impl AgentGraph for TestGraph {
        type Config = TestGraphConfig;

        fn name(&self) -> &str {
            "test"
        }

        fn resolve(
            &self,
            _context: ResolvedContext,
            _config: Self::Config,
        ) -> Result<ResolvedGraph, BlocksError> {
            Ok(ResolvedGraph {
                name: AgentGraph::name(self).to_string(),
                contribution: GenerationContribution::new(),
                integrations: Vec::new(),
            })
        }
    }

    #[derive(Debug, Clone, Deserialize, Serialize, Default)]
    struct TestProtocolConfig {}

    struct TestProtocol;

    impl Protocol for TestProtocol {
        type Config = TestProtocolConfig;

        fn name(&self) -> &str {
            "test"
        }

        fn resolve(
            &self,
            _context: ResolvedContext,
            _config: Self::Config,
        ) -> Result<ResolvedProtocol, BlocksError> {
            Ok(ResolvedProtocol {
                name: Protocol::name(self).to_string(),
                contribution: GenerationContribution::new(),
            })
        }
    }

    fn context() -> ResolvedContext {
        serde_json::from_value::<ResolvedContext>(json!({
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
    fn catalog_builder_forwards_registered_components() {
        let catalog = CompilationCatalog::builder()
            .with_archetype(TestArchetype)
            .with_graph(TestGraph)
            .with_protocol(TestProtocol)
            .build()
            .unwrap();

        let graph = catalog
            .graphs()
            .resolve("test", context(), json!({}))
            .unwrap();
        let protocol = catalog
            .protocols()
            .resolve("test", context(), json!({}))
            .unwrap();

        assert_eq!(graph.name, "test");
        assert_eq!(protocol.name, "test");
    }

    #[test]
    fn catalog_builder_supports_prebuilt_resolvers() {
        let catalog = CompilationCatalog::builder()
            .archetype_resolver(
                ArchetypeResolver::builder()
                    .with_archetype(TestArchetype)
                    .build()
                    .unwrap(),
            )
            .graph_resolver(
                GraphResolver::builder()
                    .with_graph(TestGraph)
                    .build()
                    .unwrap(),
            )
            .protocol_resolver(
                ProtocolResolver::builder()
                    .with_protocol(TestProtocol)
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap();

        assert!(catalog
            .graphs()
            .resolve(
                "test",
                context(),
                json!({})
            )
            .is_ok());
    }
}
