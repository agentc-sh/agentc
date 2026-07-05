// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use proc_macro2::TokenStream;
use serde::Serialize;
use std::marker::PhantomData;

use crate::generator::{
    blocks::{codegen::traits::CodeGen, traits::Block},
    context::GenerationContext,
    errors::GeneratorError,
    extension::{Contribution, ExtensionPoint, ExtensionRegistry},
    vfs::VirtualFileSystem,
};

pub struct CodeGenBlock<T>
where
    T: Serialize + Send + Sync,
{
    id: String,
    extension_points: Vec<ExtensionPoint>,
    contributions: Vec<Contribution>,
    codegen: Box<dyn CodeGen<T>>,
}

impl<T> CodeGenBlock<T>
where
    T: Serialize + Send + Sync,
{
    pub fn builder() -> CodeGenBlockBuilder<T> {
        CodeGenBlockBuilder::new()
    }

    fn format_pretty(stream: TokenStream) -> Result<String, GeneratorError> {
        let file = syn::parse2::<syn::File>(stream).map_err(|e| GeneratorError::Unexpected {
            message: "failed to parse generated code for formatting".to_string(),
            source: Some(Box::new(e)),
        })?;

        Ok(prettyplease::unparse(&file))
    }
}

#[async_trait]
impl<T> Block<T> for CodeGenBlock<T>
where
    T: Serialize + Send + Sync,
{
    fn id(&self) -> &str {
        &self.id
    }

    fn extension_points(&self) -> Vec<ExtensionPoint> {
        self.extension_points.clone()
    }

    fn contributions(&self) -> Vec<Contribution> {
        self.contributions.clone()
    }

    async fn render_contribution(
        &self,
        ctx: &GenerationContext<T>,
        point: &str,
    ) -> Result<String, GeneratorError> {
        self.codegen
            .generate_contribution(ctx, point)
            .map(|stream| stream.to_string())
    }

    async fn render(
        &self,
        ctx: &GenerationContext<T>,
        registry: &ExtensionRegistry,
        vfs: &mut VirtualFileSystem,
    ) -> Result<(), GeneratorError> {
        for (path, stream) in self
            .codegen
            .generate_files(ctx, registry)?
        {
            vfs.insert(path, Self::format_pretty(stream)?);
        }
        Ok(())
    }
}

pub struct CodeGenBlockBuilder<T>
where
    T: Serialize + Send + Sync,
{
    id: Option<String>,
    extension_points: Vec<ExtensionPoint>,
    contributions: Vec<Contribution>,
    _marker: PhantomData<T>,
}

impl<T> Default for CodeGenBlockBuilder<T>
where
    T: Serialize + Send + Sync,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T> CodeGenBlockBuilder<T>
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
            .push(ExtensionPoint::new(name, reducer));
        self
    }

    pub fn contribute(mut self, contribution: Contribution) -> Self {
        self.contributions.push(contribution);
        self
    }

    pub fn build<C>(self, codegen: C) -> CodeGenBlock<T>
    where
        C: CodeGen<T> + 'static,
    {
        CodeGenBlock {
            id: self
                .id
                .expect("CodeGenBlock must have a non-empty id"),
            extension_points: self.extension_points,
            contributions: self.contributions,
            codegen: Box::new(codegen),
        }
    }
}
