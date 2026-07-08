// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};
use std::{
    collections::{HashSet, hash_set::Iter},
    str::FromStr,
};

use crate::graph::errors::GraphError;

/// A single named capability, used to gate access to tools.
///
/// Capabilities follow a hierarchical naming convention using colon-separated
/// segments (e.g. `"filesystem::read"`). A granted capability covers any
/// requirement that shares its prefix (e.g. `"filesystem"` covers
/// `"filesystem::read"` and `"filesystem::write"`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Capability(String);

impl Capability {
    /// Creates a new [`Capability`] with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    /// Returns the name of the capability as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }

    /// Returns `true` if this capability satisfies the given requirement.
    ///
    /// A capability satisfies a requirement if it is exactly equal to it,
    /// or if the requirement starts with this capability followed by `"::"`.
    /// This implements hierarchical prefix matching.
    pub fn satisfies(&self, required: &Capability) -> bool {
        self.as_str() == required.as_str()
            || required
                .as_str()
                .starts_with(&format!("{}::", self.as_str()))
    }
}

impl<S: Into<String>> From<S> for Capability {
    fn from(value: S) -> Self {
        Self::new(value)
    }
}

/// A set of [`Capability`] values, used to represent what an agent has been
/// granted or what a tool requires.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct CapabilitySet(HashSet<Capability>);

impl CapabilitySet {
    /// Creates a new [`CapabilitySet`] from an iterable of items convertible to [`Capability`].
    pub fn new<I, C>(values: I) -> Self
    where
        I: IntoIterator<Item = C>,
        C: Into<Capability>,
    {
        Self(
            values
                .into_iter()
                .map(Into::into)
                .collect(),
        )
    }

    /// Creates an empty [`CapabilitySet`].
    pub fn empty() -> Self {
        Self(HashSet::new())
    }

    /// Returns a reference to the inner hash set of capabilities.
    pub fn as_inner(&self) -> &HashSet<Capability> {
        &self.0
    }

    /// Returns a mutable reference to the inner hash set of capabilities.
    pub fn as_inner_mut(&mut self) -> &mut HashSet<Capability> {
        &mut self.0
    }

    /// Consumes this set and returns the inner hash set of capabilities.
    pub fn into_inner(self) -> HashSet<Capability> {
        self.0
    }

    /// Returns an iterator over the capabilities in this set.
    pub fn iter(&self) -> Iter<'_, Capability> {
        self.as_inner().iter()
    }

    /// Returns `true` if this set is empty.
    pub fn is_empty(&self) -> bool {
        self.as_inner().is_empty()
    }

    /// Inserts a capability into this set.
    pub fn insert(&mut self, capability: Capability) {
        self.as_inner_mut().insert(capability);
    }

    /// Extends this set with capabilities from an iterable of items convertible to [`Capability`].
    pub fn extend<I, C>(&mut self, capabilities: I)
    where
        I: IntoIterator<Item = C>,
        C: Into<Capability>,
    {
        for capability in capabilities {
            self.insert(capability.into());
        }
    }

    /// Checks if this set contains specific capabilities.
    pub fn has_any(&self, values: &[Capability]) -> bool {
        values
            .iter()
            .any(|v| self.as_inner().contains(v))
    }

    /// Checks if this set contains a specific capability.
    pub fn has(&self, value: &Capability) -> bool {
        self.as_inner().contains(value)
    }

    /// Returns a new [`CapabilitySet`] containing all capabilities from both sets.
    pub fn union(&self, other: &CapabilitySet) -> Self {
        Self(
            self.as_inner()
                .union(other.as_inner())
                .cloned()
                .collect(),
        )
    }

    /// Returns `true` if every capability in `required` is satisfied by at
    /// least one capability in this set.
    pub fn satisfies_all(&self, required: &CapabilitySet) -> bool {
        required.0.iter().all(|req| {
            self.0
                .iter()
                .any(|granted| granted.satisfies(req))
        })
    }
}

impl<I, C> From<I> for CapabilitySet
where
    I: IntoIterator<Item = C>,
    C: Into<Capability>,
{
    fn from(iter: I) -> Self {
        Self::new(iter)
    }
}

/// Controls whether a thread may override the agent's base capability set.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[derive(Default)]
pub enum CapabilityOverride {
    /// Use the agent's base capability set unchanged.
    #[default]
    Inherit,
    /// Extend the agent's base capability set with additional capabilities.
    Extend(CapabilitySet),
    /// Replace the agent's base capability set entirely.
    Replace(CapabilitySet),
}

/// Determines whether per-thread capability overrides are permitted.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum CapabilityPolicy {
    /// The agent's base capability set is fixed. Any non-`Inherit` override
    /// on a thread is a hard error.
    #[default]
    Locked,
    /// Threads may extend or replace the agent's base capability set.
    Extensible,
}

impl CapabilityPolicy {
    /// Computes the effective [`CapabilitySet`] for a run, given the agent's
    /// base set and the per-thread override.
    pub fn effective(
        &self,
        base: &CapabilitySet,
        override_: &CapabilityOverride,
    ) -> Result<CapabilitySet, GraphError> {
        match self {
            Self::Locked => match override_ {
                CapabilityOverride::Inherit => Ok(base.clone()),
                _ => Err(GraphError::execution_error_message(
                    "capability override provided but agent policy is Locked",
                )),
            },
            Self::Extensible => match override_ {
                CapabilityOverride::Inherit => Ok(base.clone()),
                CapabilityOverride::Extend(extra) => Ok(base.union(extra)),
                CapabilityOverride::Replace(caps) => Ok(caps.clone()),
            },
        }
    }
}

impl FromStr for CapabilityPolicy {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "locked" => Ok(Self::Locked),
            "extensible" => Ok(Self::Extensible),
            _ => Err(format!("invalid capability policy: {}", s)),
        }
    }
}
