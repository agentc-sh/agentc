// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use thiserror::Error;

use agentc_blocks::{context::ResolvedContext, runtime::EmbeddedAsset};
use agentc_compiler::{
    generator::{
        blocks::Block, errors::GeneratorError, pipeline::Generator, vfs::VirtualFileSystem,
    },
    toolchain::traits::ErasedToolchain,
    transformer::types::TransformedAsset,
};

use crate::pipeline::{sender::Tx, steps::compose::ComposeStepOutput, traits::Step};

#[derive(Debug, Error)]
pub enum GenerateStepError {
    #[error("generator error: {0}")]
    Generator(#[from] GeneratorError),

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
    pub graph_name: String,
    pub context: ResolvedContext,
    pub blocks: Vec<Box<dyn Block<ResolvedContext>>>,
    pub toolchain: Box<dyn ErasedToolchain>,
    pub assets: Vec<TransformedAsset>,
    pub embedded_assets: Vec<&'static EmbeddedAsset>,
}

impl From<ComposeStepOutput> for GenerateStepInput {
    fn from(output: ComposeStepOutput) -> Self {
        GenerateStepInput {
            agent_name: output.agent_name,
            archetype_name: output.archetype_name,
            graph_name: output.graph_name,
            context: output.context,
            blocks: output.blocks,
            toolchain: output.toolchain,
            assets: output.assets,
            embedded_assets: output.embedded_assets,
        }
    }
}

pub struct GenerateStepOutput {
    pub agent_name: String,
    pub archetype_name: String,
    pub graph_name: String,
    pub vfs: VirtualFileSystem,
    pub toolchain: Box<dyn ErasedToolchain>,
    pub assets: Vec<TransformedAsset>,
    pub embedded_assets: Vec<&'static EmbeddedAsset>,
}

pub struct GenerateStep;

impl GenerateStep {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GenerateStep {
    fn default() -> Self {
        Self::new()
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
        tx.send(GenerateStepEvent::Started { block_count: input.blocks.len() })
            .await
            .map_err(|_| GenerateStepError::EventChannelClosed)?;

        let vfs = Generator::builder()
            .with_blocks(input.blocks)
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
            graph_name: input.graph_name,
            vfs,
            toolchain: input.toolchain,
            assets: input.assets,
            embedded_assets: input.embedded_assets,
        })
    }
}
