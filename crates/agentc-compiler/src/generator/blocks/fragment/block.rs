// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use serde::Serialize;
use std::marker::PhantomData;

use crate::generator::{
    blocks::{fragment::traits::Fragment, traits::Block},
    context::GenerationContext,
    errors::GeneratorError,
    extension::{
        Contribution, ErasedContribution, ErasedContributionValue, ErasedExtensionPoint,
        ExtensionPoint, ExtensionRegistry, StringExtensionPoint,
    },
    vfs::VirtualFileSystem,
};

pub struct FragmentBlock<T>
where
    T: Serialize + Send + Sync,
{
    id: String,
    extension_points: Vec<Box<dyn ErasedExtensionPoint>>,
    contributions: Vec<ErasedContribution>,
    fragment: Box<dyn Fragment<T>>,
}

impl<T> FragmentBlock<T>
where
    T: Serialize + Send + Sync,
{
    pub fn builder() -> FragmentBlockBuilder<T> {
        FragmentBlockBuilder::new()
    }
}

#[async_trait]
impl<T> Block<T> for FragmentBlock<T>
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
        _ctx: &GenerationContext<T>,
        _registry: &ExtensionRegistry,
        _vfs: &mut VirtualFileSystem,
    ) -> Result<(), GeneratorError> {
        Ok(())
    }
}

pub struct FragmentBlockBuilder<T>
where
    T: Serialize + Send + Sync,
{
    id: Option<String>,
    extension_points: Vec<Box<dyn ErasedExtensionPoint>>,
    contributions: Vec<ErasedContribution>,
    _marker: PhantomData<T>,
}

impl<T> Default for FragmentBlockBuilder<T>
where
    T: Serialize + Send + Sync,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T> FragmentBlockBuilder<T>
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

    pub fn build<F>(self, fragment: F) -> FragmentBlock<T>
    where
        F: Fragment<T> + 'static,
    {
        FragmentBlock {
            id: self
                .id
                .expect("FragmentBlock must have a non-empty id"),
            extension_points: self.extension_points,
            contributions: self.contributions,
            fragment: Box::new(fragment),
        }
    }
}
