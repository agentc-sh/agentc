// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use serde::Serialize;
use serde_json::Value as JsonValue;
use std::collections::{BTreeMap, HashMap};

use crate::generator::{
    blocks::{
        template::{
            evaluator::ConditionEvaluator, manifest::TemplateBlockManifest,
            renderer::TemplateRenderer,
        },
        traits::Block,
    },
    context::GenerationContext,
    errors::GeneratorError,
    extension::{Contribution, ExtensionPoint, ExtensionRegistry},
    vfs::VirtualFileSystem,
};

/// A data-driven template block implementation.
pub struct TemplateBlock {
    manifest: TemplateBlockManifest,
    templates: HashMap<String, String>,
    extra_vars: BTreeMap<String, JsonValue>,
}

impl TemplateBlock {
    pub fn new(
        manifest: TemplateBlockManifest,
        templates: HashMap<String, String>,
        extra_vars: BTreeMap<String, JsonValue>,
    ) -> Self {
        Self { manifest, templates, extra_vars }
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

    fn extension_points(&self) -> Vec<ExtensionPoint> {
        self.manifest
            .extension_points
            .iter()
            .map(|point| ExtensionPoint::new(&point.name, point.reducer.as_fn()))
            .collect()
    }

    fn contributions(&self) -> Vec<Contribution> {
        self.manifest
            .slot_fills
            .iter()
            .map(|fill| {
                if fill.strict {
                    Contribution::strict(&fill.point)
                } else {
                    Contribution::lenient(&fill.point)
                }
            })
            .collect()
    }

    async fn render_contribution(
        &self,
        ctx: &GenerationContext<T>,
        point: &str,
    ) -> Result<String, GeneratorError> {
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
            && !evaluator.evaluate(&self.manifest.id, cond)? {
                return Ok(String::new());
            }

        let template_src = self
            .get_template(&fill.template)
            .ok_or_else(|| GeneratorError::TemplateNotFound {
                block_id: self.manifest.id.clone(),
                template: fill.template.clone(),
            })?;

        renderer.render(
            &self.manifest.id,
            &fill.template,
            template_src,
            &ExtensionRegistry::empty(),
        )
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
                && !evaluator.evaluate(&self.manifest.id, cond)? {
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

pub struct TemplateBlockBuilder {
    manifest: Option<TemplateBlockManifest>,
    templates: HashMap<String, String>,
    extra_vars: BTreeMap<String, JsonValue>,
}

impl Default for TemplateBlockBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TemplateBlockBuilder {
    pub fn new() -> Self {
        Self {
            manifest: None,
            templates: HashMap::new(),
            extra_vars: BTreeMap::new(),
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

    pub fn build(self) -> TemplateBlock {
        TemplateBlock::new(
            self.manifest
                .expect("TemplateBlock requires a manifest"),
            self.templates,
            self.extra_vars,
        )
    }
}
