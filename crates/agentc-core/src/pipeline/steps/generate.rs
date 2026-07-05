// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use thiserror::Error;

use agentc_blocks::{
    archetype::resolver::ArchetypeResolver,
    composition::GenerationContribution,
    context::ResolvedContext,
    errors::BlocksError,
    runtime::EmbeddedAsset,
};
use agentc_compiler::{
    compiler::Compiler,
    generator::{
        blocks::Block, errors::GeneratorError, pipeline::Generator, vfs::VirtualFileSystem,
    },
    transformer::types::TransformedAsset,
};

use crate::{
    manifest::errors::ManifestError,
    pipeline::{sender::Tx, steps::resolve::ResolveStepOutput, traits::Step},
};

#[derive(Debug, Error)]
pub enum GenerateStepError {
    #[error("generator error: {0}")]
    Generator(#[from] GeneratorError),

    #[error("manifest error: {0}")]
    Manifest(#[from] ManifestError),

    #[error("blocks error: {0}")]
    Blocks(#[from] BlocksError),

    #[error("event channel closed")]
    EventChannelClosed,
}

pub enum GenerateStepEvent {
    Started { block_count: usize },
    Completed { vfs: VirtualFileSystem },
}

pub struct GenerateStepInput {
    pub agent_name: String,
    pub archetype_name: String,
    pub context: ResolvedContext,
    pub archetype_config: Value,
    pub assets: Vec<TransformedAsset>,
}

impl From<ResolveStepOutput> for GenerateStepInput {
    fn from(output: ResolveStepOutput) -> Self {
        GenerateStepInput {
            agent_name: output.agent_name,
            archetype_name: output.archetype_name,
            context: output.context,
            archetype_config: output.archetype_config,
            assets: output.assets,
        }
    }
}

pub struct GenerateStepOutput {
    pub agent_name: String,
    pub archetype_name: String,
    pub vfs: VirtualFileSystem,
    pub compiler: Box<dyn Compiler>,
    pub assets: Vec<TransformedAsset>,
    pub embedded_assets: Vec<&'static EmbeddedAsset>,
}

pub struct GenerateStep {
    resolver: Arc<ArchetypeResolver>,
    blocks: Vec<Box<dyn Block<ResolvedContext>>>,
}

impl GenerateStep {
    pub fn new(
        resolver: Arc<ArchetypeResolver>,
        blocks: Vec<Box<dyn Block<ResolvedContext>>>,
    ) -> Self {
        Self { resolver, blocks }
    }
}

#[async_trait]
impl Step for GenerateStep {
    type Input = GenerateStepInput;
    type Output = GenerateStepOutput;
    type Event = GenerateStepEvent;
    type Error = GenerateStepError;

    async fn execute<S>(self, input: Self::Input, tx: S) -> Result<Self::Output, Self::Error>
    where
        S: Tx<Item = Self::Event> + Send,
    {
        tx.send(GenerateStepEvent::Started { block_count: self.blocks.len() })
            .await
            .map_err(|_| GenerateStepError::EventChannelClosed)?;

        let archetype = self
            .resolver
            .resolve(&input.archetype_name, input.context.clone(), input.archetype_config)?;

        let GenerationContribution {
            mut blocks,
            embedded_assets,
            ..
        } = archetype.contribution;

        blocks.extend(self.blocks);

        let vfs = Generator::builder()
            .with_blocks(blocks)
            .with_context(input.context)
            .build()
            .generate()
            .await?;

        tx.send(GenerateStepEvent::Completed { vfs: vfs.clone() })
            .await
            .map_err(|_| GenerateStepError::EventChannelClosed)?;

        Ok(GenerateStepOutput {
            agent_name: input.agent_name,
            archetype_name: input.archetype_name,
            vfs,
            compiler: archetype.compiler,
            assets: input.assets,
            embedded_assets,
        })
    }
}
