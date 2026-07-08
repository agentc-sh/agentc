// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use serde_json::Value;
use std::collections::HashMap;

use crate::{
    archetype::{
        traits::{Archetype, ErasedArchetype},
        types::ResolvedArchetype,
    },
    context::ResolvedContext,
    errors::BlocksError,
};

/// Selects and invokes the correct [`Archetype`](crate::archetype::traits::Archetype) for a given
/// name and context.
pub struct ArchetypeResolver {
    archetypes: HashMap<String, Box<dyn ErasedArchetype>>,
}

impl ArchetypeResolver {
    /// Create a new [`ArchetypeResolver`](crate::archetype::resolver::ArchetypeResolver)
    /// with the provided map of archetype names to implementations.
    pub fn new(archetypes: HashMap<String, Box<dyn ErasedArchetype>>) -> Self {
        Self { archetypes }
    }

    /// Start building a new [`ArchetypeResolver`](crate::archetype::resolver::ArchetypeResolver) using the
    /// fluent builder API.
    pub fn builder() -> ArchetypeResolverBuilder {
        ArchetypeResolverBuilder::default()
    }

    /// Register a new archetype implementation with this resolver.
    pub fn register<T>(&mut self, archetype: T) -> Result<(), BlocksError>
    where
        T: Archetype + 'static,
    {
        let name = archetype.name().to_string();

        if self.archetypes.contains_key(&name) {
            return Err(BlocksError::duplicate_registration("archetype", name));
        }

        self.archetypes
            .insert(name, Box::new(archetype));

        Ok(())
    }

    /// Resolve the archetype with the given name using the provided context, returning the
    /// ordered list of blocks to generate.
    pub fn resolve(
        &self,
        archetype_name: &str,
        context: ResolvedContext,
        config: Value,
    ) -> Result<ResolvedArchetype, BlocksError> {
        self.archetypes
            .get(archetype_name)
            .ok_or_else(|| BlocksError::UnknownArchetype(archetype_name.to_string()))?
            .resolve_erased(context, config)
    }
}

/// Builder for [`ArchetypeResolver`](crate::archetype::resolver::ArchetypeResolver) to allow fluent
/// registration of multiple archetypes.
pub struct ArchetypeResolverBuilder {
    archetypes: HashMap<String, Box<dyn ErasedArchetype>>,
    error: Option<BlocksError>,
}

impl ArchetypeResolverBuilder {
    /// Create a new empty builder.
    pub fn new() -> Self {
        Self {
            archetypes: HashMap::new(),
            error: None,
        }
    }

    /// Add a archetype implementation to the builder.
    pub fn with_archetype<T>(mut self, archetype: T) -> Self
    where
        T: Archetype + 'static,
    {
        let name = archetype.name().to_string();

        if self.archetypes.contains_key(&name) {
            self.error = Some(BlocksError::duplicate_registration("archetype", name));
            return self;
        }

        self.archetypes
            .insert(name, Box::new(archetype));

        self
    }

    /// Finalize the builder and create a [`ArchetypeResolver`](crate::archetype::resolver::ArchetypeResolver) with the
    /// registered archetypes.
    pub fn build(self) -> Result<ArchetypeResolver, BlocksError> {
        if let Some(error) = self.error {
            return Err(error);
        }

        Ok(ArchetypeResolver::new(self.archetypes))
    }
}

impl Default for ArchetypeResolverBuilder {
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
        context::ResolvedContext,
    };

    #[derive(Debug, Clone, Deserialize, Serialize, Default)]
    struct TestArchetypeConfig;

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
            Err(BlocksError::unexpected("not needed for resolver tests"))
        }
    }

    #[test]
    fn builder_rejects_duplicate_archetype_registration() {
        let result = ArchetypeResolver::builder()
            .with_archetype(TestArchetype)
            .with_archetype(TestArchetype)
            .build();

        assert!(matches!(
            result,
            Err(BlocksError::DuplicateRegistration {
                component: "archetype",
                ..
            })
        ));
    }

    #[test]
    fn resolver_rejects_unknown_archetype() {
        let resolver = ArchetypeResolver::builder()
            .with_archetype(TestArchetype)
            .build()
            .unwrap();
        let context = serde_json::from_value::<ResolvedContext>(json!({
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
        .unwrap();
        let result = resolver
            .resolve("missing", context, json!({}));

        assert!(matches!(
            result,
            Err(BlocksError::UnknownArchetype(_))
        ));
    }
}
