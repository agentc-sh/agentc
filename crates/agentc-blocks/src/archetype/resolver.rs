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
    pub fn register<T>(&mut self, archetype: T)
    where
        T: Archetype + 'static,
    {
        self.archetypes
            .insert(archetype.name().to_string(), Box::new(archetype));
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
}

impl ArchetypeResolverBuilder {
    /// Create a new empty builder.
    pub fn new() -> Self {
        Self { archetypes: HashMap::new() }
    }

    /// Add a archetype implementation to the builder.
    pub fn with_archetype<T>(mut self, archetype: T) -> Self
    where
        T: Archetype + 'static,
    {
        self.archetypes
            .insert(archetype.name().to_string(), Box::new(archetype));
        self
    }

    /// Finalize the builder and create a [`ArchetypeResolver`](crate::archetype::resolver::ArchetypeResolver) with the
    /// registered archetypes.
    pub fn build(self) -> ArchetypeResolver {
        ArchetypeResolver::new(self.archetypes)
    }
}

impl Default for ArchetypeResolverBuilder {
    fn default() -> Self {
        Self::new()
    }
}
