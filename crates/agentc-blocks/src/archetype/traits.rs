// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use serde::de::DeserializeOwned;
use serde_json::{Value, from_value};

use crate::{archetype::types::ResolvedArchetype, context::ResolvedContext, errors::BlocksError};

/// A trait representing a build archetype, which defines how to resolve a manifest into a concrete list of blocks.
pub trait Archetype: Send + Sync {
    type Config: DeserializeOwned;

    /// The unique name used to select this archetype from the manifest's
    /// `build { archetype = "..." }` field.
    fn name(&self) -> &str;

    /// Assemble the full ordered block list from the provided context.
    fn resolve(
        &self,
        context: ResolvedContext,
        config: Self::Config,
    ) -> Result<ResolvedArchetype, BlocksError>;
}

/// An archetype object with its type erased, for use in dynamic dispatch.
pub trait ErasedArchetype: Send + Sync {
    fn name(&self) -> &str;
    fn resolve_erased(
        &self,
        context: ResolvedContext,
        config: Value,
    ) -> Result<ResolvedArchetype, BlocksError>;
}

impl<T> ErasedArchetype for T
where
    T: Archetype + Send + Sync,
    T::Config: 'static,
{
    fn name(&self) -> &str {
        self.name()
    }

    fn resolve_erased(
        &self,
        context: ResolvedContext,
        config: Value,
    ) -> Result<ResolvedArchetype, BlocksError> {
        self.resolve(context, from_value(config)?)
    }
}
