// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use std::path::Path;

use crate::{
    asset::types::AssetOrigin,
    transformer::{errors::TransformError, types::AssetArtifact},
};

/// Receives subprocess output lines emitted by a transformer during asset processing.
#[async_trait]
pub trait TransformSink: Send + Sync {
    async fn stdout(&self, line: &str);
    async fn stderr(&self, line: &str);
}

pub struct NoopTransformSink;

#[async_trait]
impl TransformSink for NoopTransformSink {
    async fn stdout(&self, _line: &str) {}
    async fn stderr(&self, _line: &str) {}
}

/// A transformer that can process a fetched asset artifact into one or more
/// labeled output artifacts.
///
/// Transformers are registered in a [`TransformerRegistry`](crate::transformer::registry::TransformerRegistry)
/// and applied in registration order. Multiple transformers may match and run
/// against the same asset, with each receiving the original fetched path.
#[async_trait]
pub trait AssetTransformer: Send + Sync {
    /// Returns `true` if this transformer should process the asset at
    /// `local_path`. May perform filesystem inspection to make this
    /// determination (e.g. checking for `package.json`, `.ts` extension, etc).
    async fn can_transform(&self, local_path: &Path, origin: &AssetOrigin) -> bool;

    /// Transform the asset at `local_path` into one or more labeled output
    /// artifacts. Each artifact carries a `kind` string identifying what it
    /// represents (e.g. `"source"`, `"venv"`). Subprocess output lines are
    /// forwarded to `sink`.
    async fn transform(
        &self,
        local_path: &Path,
        origin: &AssetOrigin,
        sink: &dyn TransformSink,
    ) -> Result<Vec<AssetArtifact>, TransformError>;
}
