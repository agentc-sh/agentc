// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use thiserror::Error;

use agentc_blocks::{
    catalog::CompilationCatalog,
    composition::{ComposedGeneration, Composer, CompositionInput},
    context::ResolvedContext,
    errors::BlocksError,
    runtime::EmbeddedAsset,
};
use agentc_compiler::{
    generator::blocks::Block, toolchain::traits::ErasedToolchain,
    transformer::types::TransformedAsset,
};

use crate::pipeline::{sender::Tx, steps::resolve::ResolveStepOutput, traits::Step};

#[derive(Debug, Error)]
pub enum ComposeStepError {
    #[error("blocks error: {0}")]
    Blocks(#[from] BlocksError),

    #[error("event channel closed")]
    EventChannelClosed,
}

pub enum ComposeStepEvent {
    Started,
    Completed {
        archetype_name: String,
        graph_name: String,
        protocol_names: Vec<String>,
        block_count: usize,
    },
}

pub struct ComposeStepInput {
    pub agent_name: String,
    pub archetype_name: String,
    pub archetype_config: Value,
    pub graph_name: String,
    pub graph_config: Value,
    pub protocol_selections: Vec<(String, Value)>,
    pub context: ResolvedContext,
    pub assets: Vec<TransformedAsset>,
}

impl From<ResolveStepOutput> for ComposeStepInput {
    fn from(output: ResolveStepOutput) -> Self {
        ComposeStepInput {
            agent_name: output.agent_name,
            archetype_name: output.archetype_name,
            archetype_config: output.archetype_config,
            graph_name: output.graph_name,
            graph_config: output.graph_config,
            protocol_selections: output.protocol_selections,
            context: output.context,
            assets: output.assets,
        }
    }
}

pub struct ComposeStepOutput {
    pub agent_name: String,
    pub archetype_name: String,
    pub graph_name: String,
    pub protocol_names: Vec<String>,
    pub context: ResolvedContext,
    pub toolchain: Box<dyn ErasedToolchain>,
    pub blocks: Vec<Box<dyn Block<ResolvedContext>>>,
    pub embedded_assets: Vec<&'static EmbeddedAsset>,
    pub assets: Vec<TransformedAsset>,
}

pub struct ComposeStep {
    catalog: Arc<CompilationCatalog>,
    blocks: Vec<Box<dyn Block<ResolvedContext>>>,
}

impl ComposeStep {
    pub fn new(
        catalog: Arc<CompilationCatalog>,
        blocks: Vec<Box<dyn Block<ResolvedContext>>>,
    ) -> Self {
        Self { catalog, blocks }
    }
}

#[async_trait]
impl Step for ComposeStep {
    type Input = ComposeStepInput;
    type Output = ComposeStepOutput;
    type Event = ComposeStepEvent;
    type Error = ComposeStepError;

    async fn execute<S>(self, input: Self::Input, tx: S) -> Result<Self::Output, Self::Error>
    where
        S: Tx<Item = Self::Event> + Send,
    {
        tx.send(ComposeStepEvent::Started)
            .await
            .map_err(|_| ComposeStepError::EventChannelClosed)?;

        let archetype = self.catalog.archetypes().resolve(
            &input.archetype_name,
            input.context.clone(),
            input.archetype_config,
        )?;
        let graph = self.catalog.graphs().resolve(
            &input.graph_name,
            input.context.clone(),
            input.graph_config,
        )?;
        let protocols = input
            .protocol_selections
            .into_iter()
            .map(|(name, config)| {
                self.catalog
                    .protocols()
                    .resolve(&name, input.context.clone(), config)
            })
            .collect::<Result<Vec<_>, BlocksError>>()?;

        let ComposedGeneration {
            archetype_name,
            graph_name,
            protocol_names,
            toolchain,
            blocks,
            embedded_assets,
        } = Composer::new().compose(CompositionInput {
            archetype,
            graph,
            protocols,
            blocks: self.blocks,
        })?;

        tx.send(ComposeStepEvent::Completed {
            archetype_name: archetype_name.clone(),
            graph_name: graph_name.clone(),
            protocol_names: protocol_names.clone(),
            block_count: blocks.len(),
        })
        .await
        .map_err(|_| ComposeStepError::EventChannelClosed)?;

        Ok(ComposeStepOutput {
            agent_name: input.agent_name,
            archetype_name,
            graph_name,
            protocol_names,
            context: input.context,
            toolchain,
            blocks,
            embedded_assets,
            assets: input.assets,
        })
    }
}
