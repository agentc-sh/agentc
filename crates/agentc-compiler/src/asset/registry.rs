// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::{collections::HashMap, path::PathBuf};

use crate::asset::handler::AssetOrigin;

#[derive(Debug, Clone)]
pub struct AssetsEntry {
    /// The local file path where the source content can be found.
    pub local_path: PathBuf,
    /// The origin of this asset.
    pub origin: AssetOrigin,
}

/// A mapping of asset URIs to their resolved local paths.
///
/// Populated by [`AssetResolver`](crate::resolver::resolver::SourceResolver)
/// during the fetch step and passed through the rest of the pipeline.
#[derive(Debug, Clone, Default)]
pub struct Assets {
    entries: HashMap<String, AssetsEntry>,
}

impl Assets {
    /// Create a new empty [`Assets`] mapping.
    pub fn new() -> Self {
        Self { entries: HashMap::new() }
    }

    /// Insert a resolved local path for the given URI.
    pub fn insert(&mut self, uri: impl Into<String>, entry: impl Into<AssetsEntry>) {
        self.entries
            .insert(uri.into(), entry.into());
    }

    /// Returns the resolved entry for the given URI, if it exists.
    pub fn get(&self, uri: &str) -> Option<&AssetsEntry> {
        self.entries.get(uri)
    }

    /// Returns `true` if the registry contains an entry for the given URI.
    pub fn contains(&self, uri: &str) -> bool {
        self.entries.contains_key(uri)
    }

    /// Returns the number of entries in the registry.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns `true` if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns an iterator over all URI and entry pairs in the registry.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &AssetsEntry)> {
        self.entries.iter()
    }

    /// Consumes the registry and returns the underlying map of entries.
    pub fn into_entries(self) -> HashMap<String, AssetsEntry> {
        self.entries
    }
}
