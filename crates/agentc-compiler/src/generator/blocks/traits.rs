// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use serde::Serialize;

use crate::generator::{
    context::GenerationContext,
    errors::GeneratorError,
    extension::{
        ErasedContribution, ErasedContributionValue, ErasedExtensionPoint, ExtensionRegistry,
    },
    vfs::VirtualFileSystem,
};

#[async_trait]
pub trait Block<T>: Send + Sync
where
    T: Serialize + Send + Sync,
{
    /// A unique identifier for this block within a generation run.
    fn id(&self) -> &str;

    /// Extension points declared by this block, where contributions from
    /// other blocks will be collected and resolved.
    fn extension_points(&self) -> Vec<Box<dyn ErasedExtensionPoint>> {
        vec![]
    }

    /// Contributions this block makes to extension points declared by other blocks.
    fn contributions(&self) -> Vec<ErasedContribution> {
        vec![]
    }

    /// Render this block's contribution for a specific extension point.
    async fn render_contribution(
        &self,
        _ctx: &GenerationContext<T>,
        _point: &str,
    ) -> Result<ErasedContributionValue, GeneratorError> {
        Ok(ErasedContributionValue::new(String::new()))
    }

    /// Render this block's content into the virtual file system.
    async fn render(
        &self,
        ctx: &GenerationContext<T>,
        registry: &ExtensionRegistry,
        vfs: &mut VirtualFileSystem,
    ) -> Result<(), GeneratorError>;
}
