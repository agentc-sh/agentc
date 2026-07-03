// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use std::path::PathBuf;

use crate::generator::errors::GeneratorError;

/// A trait for loading resources (e.g., templates, context data) by path.
#[async_trait]
pub trait ResourceLoader: Send + Sync {
    /// Fetch the UTF-8 content of the resource identified by the given path.
    async fn load(&self, path: &str) -> Result<String, GeneratorError>;
}

/// Resolves resource paths relative to a base directory on the local filesystem.
pub struct FileSystemLoader {
    base_dir: PathBuf,
}

impl FileSystemLoader {
    /// Create a loader that resolves paths relative to `base_dir`.
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self { base_dir: base_dir.into() }
    }
}

#[async_trait]
impl ResourceLoader for FileSystemLoader {
    async fn load(&self, path: &str) -> Result<String, GeneratorError> {
        tokio::fs::read_to_string(&self.base_dir.join(path))
            .await
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    GeneratorError::resource_not_found(path)
                } else {
                    GeneratorError::resource_load_failed(path, e.to_string())
                }
            })
    }
}
