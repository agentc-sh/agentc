// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::path::PathBuf;

use crate::asset::types::AssetOrigin;

/// The payload of an [`AssetArtifact`]: either a path on the filesystem or an
/// in-memory string value. An artifact is one or the other, never both.
#[derive(Debug, Clone)]
pub enum AssetArtifactContent {
    /// A file or directory on the local filesystem.
    Path(PathBuf),
    /// A small in-memory value (e.g. a package name, a version string).
    Value(String),
}

/// A labeled output artifact produced by an [`AssetTransformer`](crate::transformer::traits::AssetTransformer).
#[derive(Debug, Clone)]
pub struct AssetArtifact {
    /// A string label identifying what this artifact represents.
    /// Conventions are defined by the transformer and consumed by the resolver.
    /// Examples: `"source"`, `"site_packages_path"`, `"module_name"`.
    pub kind: String,
    /// The artifact payload: a filesystem path or an in-memory value.
    pub content: AssetArtifactContent,
    /// When `true`, this artifact's path should be deleted after compilation completes.
    /// Used for temporary directories (e.g. isolated venvs) that are only needed during
    /// the build and should not persist afterward.
    pub ephemeral: bool,
}

impl AssetArtifact {
    /// Create a path artifact.
    pub fn path(kind: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self {
            kind: kind.into(),
            content: AssetArtifactContent::Path(path.into()),
            ephemeral: false,
        }
    }

    /// Create an ephemeral path artifact that will be deleted after compilation.
    pub fn ephemeral_path(kind: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self {
            kind: kind.into(),
            content: AssetArtifactContent::Path(path.into()),
            ephemeral: true,
        }
    }

    /// Create an in-memory value artifact.
    pub fn value(kind: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            content: AssetArtifactContent::Value(value.into()),
            ephemeral: false,
        }
    }

    /// Returns the path if this is a path artifact.
    pub fn as_path(&self) -> Option<&PathBuf> {
        match &self.content {
            AssetArtifactContent::Path(p) => Some(p),
            AssetArtifactContent::Value(_) => None,
        }
    }

    /// Returns the value string if this is an in-memory value artifact.
    pub fn as_value(&self) -> Option<&str> {
        match &self.content {
            AssetArtifactContent::Path(_) => None,
            AssetArtifactContent::Value(v) => Some(v.as_str()),
        }
    }
}

/// An asset that has been processed by the transform step, carrying one or
/// more labeled output artifacts.
#[derive(Debug, Clone)]
pub struct TransformedAsset {
    /// The original URI this asset was fetched from.
    pub uri: String,
    /// The origin of this asset, for error attribution.
    pub origin: AssetOrigin,
    /// The labeled output artifacts produced by the transformer(s).
    pub artifacts: Vec<AssetArtifact>,
}

impl TransformedAsset {
    /// Find an artifact by kind, returning the first match.
    pub fn artifact(&self, kind: &str) -> Option<&AssetArtifact> {
        self.artifacts
            .iter()
            .find(|a| a.kind == kind)
    }

    /// Returns all artifacts of the given kind.
    pub fn artifacts_of(&self, kind: &str) -> Vec<&AssetArtifact> {
        self.artifacts
            .iter()
            .filter(|a| a.kind == kind)
            .collect()
    }
}
