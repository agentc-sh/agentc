// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use serde::Serialize;
use serde_json::Value as JsonValue;
use std::{
    collections::{BTreeMap, HashMap},
    marker::PhantomData,
};

use crate::generator::{
    blocks::{
        template::{
            evaluator::ConditionEvaluator, manifest::TemplateBlockManifest,
            renderer::TemplateRenderer, traits::TemplateFragment,
        },
        traits::Block,
    },
    context::GenerationContext,
    errors::GeneratorError,
    extension::{
        Contribution, ErasedContribution, ErasedContributionValue, ErasedExtensionPoint,
        ExtensionPoint, ExtensionRegistry, StringExtensionPoint,
    },
    vfs::VirtualFileSystem,
};

/// A data-driven template block implementation.
pub struct TemplateBlock {
    manifest: TemplateBlockManifest,
    templates: HashMap<String, String>,
    extra_vars: BTreeMap<String, JsonValue>,
    extra_extension_points: Vec<Box<dyn ErasedExtensionPoint>>,
}

impl TemplateBlock {
    pub fn new(
        manifest: TemplateBlockManifest,
        templates: HashMap<String, String>,
        extra_vars: BTreeMap<String, JsonValue>,
        extra_extension_points: Vec<Box<dyn ErasedExtensionPoint>>,
    ) -> Self {
        Self {
            manifest,
            templates,
            extra_vars,
            extra_extension_points,
        }
    }

    fn renderer<T: Serialize>(&self, data: &T) -> TemplateRenderer {
        if self.extra_vars.is_empty() {
            TemplateRenderer::new(data)
        } else {
            TemplateRenderer::with_vars(data, self.extra_vars.clone())
        }
    }

    pub fn builder() -> TemplateBlockBuilder {
        TemplateBlockBuilder::new()
    }

    fn get_template(&self, name: &str) -> Option<&str> {
        self.templates
            .get(name)
            .map(String::as_str)
    }
}

#[async_trait]
impl<T> Block<T> for TemplateBlock
where
    T: Serialize + Send + Sync,
{
    fn id(&self) -> &str {
        &self.manifest.id
    }

    fn extension_points(&self) -> Vec<Box<dyn ErasedExtensionPoint>> {
        self.manifest
            .extension_points
            .iter()
            .map(|point| {
                Box::new(StringExtensionPoint::new(&point.name, point.reducer.as_fn()))
                    as Box<dyn ErasedExtensionPoint>
            })
            .chain(self.extra_extension_points.clone())
            .collect()
    }

    fn contributions(&self) -> Vec<ErasedContribution> {
        self.manifest
            .slot_fills
            .iter()
            .map(|fill| {
                if fill.strict {
                    Contribution::<String>::strict(&fill.point).erase()
                } else {
                    Contribution::<String>::lenient(&fill.point).erase()
                }
            })
            .collect()
    }

    async fn render_contribution(
        &self,
        ctx: &GenerationContext<T>,
        point: &str,
    ) -> Result<ErasedContributionValue, GeneratorError> {
        let evaluator = ConditionEvaluator::new(ctx.as_inner())?;
        let renderer = self.renderer(ctx.as_inner());

        let fill = self
            .manifest
            .slot_fills
            .iter()
            .find(|sf| sf.point == point)
            .ok_or_else(|| GeneratorError::TemplateNotFound {
                block_id: self.manifest.id.clone(),
                template: point.to_string(),
            })?;

        if let Some(cond) = &fill.condition
            && !evaluator.evaluate(&self.manifest.id, cond)?
        {
            return Ok(ErasedContributionValue::new(String::new()));
        }

        let template_src = self
            .get_template(&fill.template)
            .ok_or_else(|| GeneratorError::TemplateNotFound {
                block_id: self.manifest.id.clone(),
                template: fill.template.clone(),
            })?;

        renderer
            .render(&self.manifest.id, &fill.template, template_src, &ExtensionRegistry::empty())
            .map(ErasedContributionValue::new)
    }

    async fn render(
        &self,
        ctx: &GenerationContext<T>,
        registry: &ExtensionRegistry,
        vfs: &mut VirtualFileSystem,
    ) -> Result<(), GeneratorError> {
        let evaluator = ConditionEvaluator::new(ctx.as_inner())?;
        let renderer = self.renderer(ctx.as_inner());

        for file_spec in &self.manifest.files {
            if let Some(cond) = &file_spec.condition
                && !evaluator.evaluate(&self.manifest.id, cond)?
            {
                continue;
            }

            let path =
                renderer.render(&self.manifest.id, &file_spec.path, &file_spec.path, registry)?;

            let template_src = self
                .get_template(&file_spec.template)
                .ok_or_else(|| GeneratorError::TemplateNotFound {
                    block_id: self.manifest.id.clone(),
                    template: file_spec.template.clone(),
                })?;

            let content =
                renderer.render(&self.manifest.id, &file_spec.template, template_src, registry)?;

            vfs.insert(path, content);
        }

        Ok(())
    }
}

impl TemplateBlockBuilder {
    pub fn new() -> Self {
        Self {
            manifest: None,
            templates: HashMap::new(),
            extra_vars: BTreeMap::new(),
            extra_extension_points: Vec::new(),
        }
    }

    pub fn with_var(mut self, name: impl Into<String>, value: impl Serialize) -> Self {
        self.extra_vars
            .insert(name.into(), serde_json::to_value(value).unwrap_or_default());
        self
    }

    pub fn with_manifest(mut self, manifest: TemplateBlockManifest) -> Self {
        self.manifest = Some(manifest);
        self
    }

    pub fn with_template<S>(mut self, name: S, src: S) -> Self
    where
        S: AsRef<str>,
    {
        self.templates
            .insert(name.as_ref().to_string(), src.as_ref().to_string());
        self
    }

    pub fn typed_extension_point<P>(mut self, point: P) -> Self
    where
        P: ExtensionPoint + Clone + 'static,
    {
        self.extra_extension_points
            .push(Box::new(point));
        self
    }

    pub fn build(self) -> TemplateBlock {
        TemplateBlock::new(
            self.manifest
                .expect("TemplateBlock requires a manifest"),
            self.templates,
            self.extra_vars,
            self.extra_extension_points,
        )
    }
}

pub struct TemplateBlockBuilder {
    manifest: Option<TemplateBlockManifest>,
    templates: HashMap<String, String>,
    extra_vars: BTreeMap<String, JsonValue>,
    extra_extension_points: Vec<Box<dyn ErasedExtensionPoint>>,
}

impl Default for TemplateBlockBuilder {
    fn default() -> Self {
        Self::new()
    }
}

pub struct TemplateFragmentBlock<T>
where
    T: Serialize + Send + Sync,
{
    id: String,
    extension_points: Vec<Box<dyn ErasedExtensionPoint>>,
    contributions: Vec<ErasedContribution>,
    fragment: Box<dyn TemplateFragment<T>>,
}

impl<T> TemplateFragmentBlock<T>
where
    T: Serialize + Send + Sync,
{
    pub fn builder() -> TemplateFragmentBlockBuilder<T> {
        TemplateFragmentBlockBuilder::new()
    }
}

#[async_trait]
impl<T> Block<T> for TemplateFragmentBlock<T>
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
        self.contributions.clone()
    }

    async fn render_contribution(
        &self,
        ctx: &GenerationContext<T>,
        point: &str,
    ) -> Result<ErasedContributionValue, GeneratorError> {
        self.fragment
            .generate_contribution(ctx, point)
    }

    async fn render(
        &self,
        ctx: &GenerationContext<T>,
        registry: &ExtensionRegistry,
        vfs: &mut VirtualFileSystem,
    ) -> Result<(), GeneratorError> {
        for (path, content) in self
            .fragment
            .generate_files(ctx, registry)?
        {
            vfs.insert(path, content);
        }

        Ok(())
    }
}

pub struct TemplateFragmentBlockBuilder<T>
where
    T: Serialize + Send + Sync,
{
    id: Option<String>,
    extension_points: Vec<Box<dyn ErasedExtensionPoint>>,
    contributions: Vec<ErasedContribution>,
    _marker: PhantomData<T>,
}

impl<T> Default for TemplateFragmentBlockBuilder<T>
where
    T: Serialize + Send + Sync,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T> TemplateFragmentBlockBuilder<T>
where
    T: Serialize + Send + Sync,
{
    pub fn new() -> Self {
        Self {
            id: None,
            extension_points: Vec::new(),
            contributions: Vec::new(),
            _marker: PhantomData,
        }
    }

    pub fn id(mut self, id: impl AsRef<str>) -> Self {
        self.id = Some(id.as_ref().to_string());
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

    pub fn typed_extension_point<P>(mut self, point: P) -> Self
    where
        P: ExtensionPoint + Clone + 'static,
    {
        self.extension_points
            .push(Box::new(point));
        self
    }

    pub fn contribute<C>(mut self, contribution: Contribution<C>) -> Self
    where
        C: Send + Sync + 'static,
    {
        self.contributions
            .push(contribution.erase());
        self
    }

    pub fn build<F>(self, fragment: F) -> TemplateFragmentBlock<T>
    where
        F: TemplateFragment<T> + 'static,
    {
        TemplateFragmentBlock {
            id: self
                .id
                .expect("TemplateFragmentBlock must have a non-empty id"),
            extension_points: self.extension_points,
            contributions: self.contributions,
            fragment: Box::new(fragment),
        }
    }
}
