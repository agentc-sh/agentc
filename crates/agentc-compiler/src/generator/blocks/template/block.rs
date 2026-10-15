// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use serde::Serialize;
use std::collections::BTreeMap;

use crate::generator::{
    blocks::{
        template::{renderer::TemplateRenderer, traits::RenderedContribution},
        traits::Block,
    },
    context::GenerationContext,
    errors::GeneratorError,
    extension::{
        Contribution, ErasedContribution, ErasedContributionValue, ErasedExtensionPoint,
        ExtensionPoint, ExtensionRegistry, StringExtensionPoint, TokenStreamExtensionPoint,
    },
    vfs::VirtualFileSystem,
};

struct TemplateFile {
    path: String,
    source: String,
}

impl TemplateFile {
    fn render(
        &self,
        block_id: &str,
        renderer: &TemplateRenderer,
        registry: &ExtensionRegistry,
        vfs: &mut VirtualFileSystem,
    ) -> Result<(), GeneratorError> {
        vfs.insert(
            renderer.render(block_id, &self.path, &self.path, registry)?,
            renderer.render(block_id, &self.path, &self.source, registry)?,
        );

        Ok(())
    }
}

struct TemplateContribution {
    contribution: ErasedContribution,
    source: String,
    value: fn(String) -> Result<ErasedContributionValue, GeneratorError>,
}

impl TemplateContribution {
    fn new<C>(contribution: Contribution<C>, source: String) -> Self
    where
        C: RenderedContribution,
    {
        Self {
            contribution: contribution.erase(),
            source,
            value: |rendered| C::from_rendered(rendered).map(ErasedContributionValue::new),
        }
    }

    fn targets(&self, point: &str) -> bool {
        self.contribution.point == point
    }

    fn erased(&self) -> ErasedContribution {
        self.contribution.clone()
    }

    fn render(
        &self,
        block_id: &str,
        renderer: &TemplateRenderer,
    ) -> Result<ErasedContributionValue, GeneratorError> {
        (self.value)(renderer.render(
            block_id,
            &self.contribution.point,
            &self.source,
            &ExtensionRegistry::empty(),
        )?)
    }
}

pub struct TemplateBlock {
    id: String,
    vars: BTreeMap<String, serde_json::Value>,
    files: Vec<TemplateFile>,
    extension_points: Vec<Box<dyn ErasedExtensionPoint>>,
    contributions: Vec<TemplateContribution>,
}

impl TemplateBlock {
    pub fn builder() -> TemplateBlockBuilder {
        TemplateBlockBuilder::new()
    }
}

#[async_trait]
impl<T> Block<T> for TemplateBlock
where
    T: Serialize + Send + Sync,
{
    fn id(&self) -> &str {
        &self.id
    }

    fn extension_points(&self) -> Vec<Box<dyn ErasedExtensionPoint>> {
        self.extension_points.clone()
    }

    fn contributions(&self) -> Vec<ErasedContribution> {
        self.contributions
            .iter()
            .map(TemplateContribution::erased)
            .collect()
    }

    async fn render_contribution(
        &self,
        ctx: &GenerationContext<T>,
        point: &str,
    ) -> Result<ErasedContributionValue, GeneratorError> {
        self.contributions
            .iter()
            .find(|contribution| contribution.targets(point))
            .ok_or_else(|| {
                GeneratorError::unexpected(format!(
                    "block '{}' does not contribute to extension point '{point}'",
                    self.id,
                ))
            })?
            .render(
                &self.id,
                &TemplateRenderer::new(ctx.as_inner())
                    .with_vars(self.vars.clone()),
            )
    }

    async fn render(
        &self,
        ctx: &GenerationContext<T>,
        registry: &ExtensionRegistry,
        vfs: &mut VirtualFileSystem,
    ) -> Result<(), GeneratorError> {
        let renderer = TemplateRenderer::new(ctx.as_inner())
            .with_vars(self.vars.clone());

        for file in &self.files {
            file.render(&self.id, &renderer, registry, vfs)?;
        }

        Ok(())
    }
}

pub struct TemplateBlockBuilder {
    id: Option<String>,
    vars: BTreeMap<String, serde_json::Value>,
    files: Vec<TemplateFile>,
    extension_points: Vec<Box<dyn ErasedExtensionPoint>>,
    contributions: Vec<TemplateContribution>,
}

impl Default for TemplateBlockBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TemplateBlockBuilder {
    pub fn new() -> Self {
        Self {
            id: None,
            vars: BTreeMap::new(),
            files: Vec::new(),
            extension_points: Vec::new(),
            contributions: Vec::new(),
        }
    }

    pub fn id(mut self, id: impl AsRef<str>) -> Self {
        self.id = Some(id.as_ref().to_string());
        self
    }

    pub fn var(mut self, name: impl Into<String>, value: impl Serialize) -> Self {
        self.vars
            .insert(name.into(), serde_json::to_value(value).unwrap_or_default());
        self
    }

    pub fn file(mut self, path: impl Into<String>, source: impl Into<String>) -> Self {
        self.files.push(TemplateFile {
            path: path.into(),
            source: source.into(),
        });
        self
    }

    pub fn extension_point(
        mut self,
        name: impl Into<String>,
        reducer: fn(Vec<String>) -> String,
    ) -> Self {
        self.extension_points
            .push(Box::new(StringExtensionPoint::new(name, reducer)));
        self
    }

    pub fn token_stream_extension_point(
        mut self,
        name: impl Into<String>,
        reducer: fn(Vec<String>) -> String,
    ) -> Self {
        self.extension_points
            .push(Box::new(TokenStreamExtensionPoint::new(name, reducer)));
        self
    }

    pub fn typed_extension_point<P>(mut self, point: P) -> Self
    where
        P: ExtensionPoint + Clone + 'static,
    {
        self.extension_points
            .push(Box::new(point));
        self
    }

    pub fn contribute<C>(mut self, contribution: Contribution<C>, source: impl Into<String>) -> Self
    where
        C: RenderedContribution,
    {
        self.contributions
            .push(TemplateContribution::new(contribution, source.into()));
        self
    }

    pub fn build(self) -> TemplateBlock {
        TemplateBlock {
            id: self
                .id
                .expect("TemplateBlock must have a non-empty id"),
            vars: self.vars,
            files: self.files,
            extension_points: self.extension_points,
            contributions: self.contributions,
        }
    }
}
