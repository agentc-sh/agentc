// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use std::path::{Path, PathBuf};

use crate::{asset::errors::AssetError, utils::symlink};

/// A reference to a asset that needs to be fetched, along with metadata
/// about what it is for.
#[derive(Debug, Clone)]
pub struct AssetRef {
    /// The URI of the asset to fetch.
    pub uri: String,
    /// The origin of the asset, for error attribution.
    pub origin: AssetOrigin,
}

impl AssetRef {
    pub fn new(uri: impl Into<String>, origin: AssetOrigin) -> Self {
        Self { uri: uri.into(), origin }
    }
}

/// Describes what a asset is for, used for attributing errors.
#[derive(Debug, Clone)]
pub enum AssetOrigin {
    /// A tool defined in the manifest.
    Tool { name: String },
    /// A block defined in the manifest.
    Block { name: String },
}

impl AssetOrigin {
    pub fn tool(name: impl Into<String>) -> Self {
        Self::Tool { name: name.into() }
    }

    pub fn block(name: impl Into<String>) -> Self {
        Self::Block { name: name.into() }
    }
}

/// A handler that can fetch a asset URI and write it to a destination path.
#[async_trait]
pub trait AssetHandler: Send + Sync {
    /// Returns `true` if this handler is able to handle the given URI.
    fn can_handle(&self, uri: &str) -> bool;

    /// Fetches the asset at `uri` and writes it to `dest`.
    async fn fetch(&self, uri: &str, dest: &Path) -> Result<(), AssetError>;
}

/// A built-in handler for local file sources.
///
/// Handles bare relative paths (e.g. `./tools/search.ts`), bare absolute
/// paths, and URIs with a `file:` scheme prefix.
pub struct LocalFileHandler {
    context_dir: PathBuf,
}

impl LocalFileHandler {
    pub fn new(context_dir: impl Into<PathBuf>) -> Self {
        Self { context_dir: context_dir.into() }
    }

    fn normalize_path<'a>(&self, uri: &'a str) -> &'a str {
        uri.strip_prefix("file:").unwrap_or(uri)
    }
}

#[async_trait]
impl AssetHandler for LocalFileHandler {
    fn can_handle(&self, uri: &str) -> bool {
        let path = self.normalize_path(uri);

        path.starts_with("./") || path.starts_with("../") || path.starts_with('/')
    }

    async fn fetch(&self, uri: &str, dest: &Path) -> Result<(), AssetError> {
        let path = tokio::fs::canonicalize(
            self.context_dir
                .join(self.normalize_path(uri)),
        )
        .await
        .map_err(|e| AssetError::io(uri, e))?;

        if let Some(parent) = dest.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| AssetError::io(uri, e))?;
        }

        // Remove a dangling symlink (target deleted) so we can replace it below.
        // symlink_metadata() does not follow symlinks, so it returns Ok for dangling ones,
        // while exists() does follow symlinks and returns false when the target is gone.
        if dest.symlink_metadata().is_ok() && !dest.exists() {
            tokio::fs::remove_file(&dest)
                .await
                .map_err(|e| AssetError::io(uri, e))?;
        }

        if dest.symlink_metadata().is_err() {
            symlink(&path, dest)
                .await
                .map_err(|e| AssetError::io(uri, e))?;
        }

        Ok(())
    }
}
