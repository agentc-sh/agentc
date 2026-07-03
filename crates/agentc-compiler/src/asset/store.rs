// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// A local directory used to cache fetched artifacts.
///
/// Artifacts are stored at deterministic paths derived from their source URI,
/// enabling cache lookups without tracking state.
pub struct ArtifactStore {
    root: PathBuf,
    force: bool,
}

impl ArtifactStore {
    /// Create a new [`ArtifactStore`](crate::resolver::store::ArtifactStore) rooted at the given directory.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into(), force: false }
    }

    /// Force re-fetching of all artifacts, bypassing the cache.
    pub fn with_force(mut self, force: bool) -> Self {
        self.force = force;
        self
    }

    /// Returns the root directory of the artifact store.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Returns whether the store is in force mode.
    pub fn is_force(&self) -> bool {
        self.force
    }

    /// Returns the deterministic local path for a given URI.
    ///
    /// The path is derived from a SHA-256 hash of the URI combined with
    /// the original filename, for human readability in the artifacts directory.
    pub fn path_for(&self, uri: &str) -> PathBuf {
        let hash = format!("{:x}", Sha256::digest(uri.as_bytes()));
        let filename = uri
            .split('/')
            .next_back()
            .filter(|s| !s.is_empty())
            .unwrap_or("artifact");

        self.root
            .join(&hash[..16])
            .join(filename)
    }

    /// Returns `true` if the artifact for the given URI is already cached.
    pub fn is_cached(&self, uri: &str) -> bool {
        !self.force && self.path_for(uri).exists()
    }
}
