// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use thiserror::Error;

use agentc_blocks::context::ResolvedContext;
use agentc_compiler::{generator::loader::ResourceLoader, transformer::types::TransformedAsset};

use crate::{
    manifest::{Manifest, errors::ManifestError},
    pipeline::{sender::Tx, steps::transform::TransformStepOutput, traits::Step},
};

#[derive(Debug, Error)]
pub enum ResolveStepError {
    #[error("manifest error: {0}")]
    Manifest(#[from] ManifestError),

    #[error("event channel closed")]
    EventChannelClosed,
}

pub enum ResolveStepEvent {
    Started,
    Completed {
        agent_name: String,
        archetype_name: String,
        tool_count: usize,
        block_count: usize,
    },
}

pub struct ResolveStepInput {
    pub manifest: Manifest,
    pub assets: Vec<TransformedAsset>,
}

impl From<TransformStepOutput> for ResolveStepInput {
    fn from(output: TransformStepOutput) -> Self {
        ResolveStepInput {
            manifest: output.manifest,
            assets: output.assets,
        }
    }
}

pub struct ResolveStepOutput {
    pub agent_name: String,
    pub archetype_name: String,
    pub archetype_config: Value,
    pub graph_name: String,
    pub graph_config: Value,
    pub protocol_selections: Vec<(String, Value)>,
    pub context: ResolvedContext,
    pub assets: Vec<TransformedAsset>,
}

pub struct ResolveStep {
    loader: Arc<dyn ResourceLoader>,
}

impl ResolveStep {
    pub fn new(loader: Arc<dyn ResourceLoader>) -> Self {
        Self { loader }
    }
}

#[async_trait]
impl Step for ResolveStep {
    type Input = ResolveStepInput;
    type Output = ResolveStepOutput;
    type Event = ResolveStepEvent;
    type Error = ResolveStepError;

    async fn execute<S>(self, input: Self::Input, tx: S) -> Result<Self::Output, Self::Error>
    where
        S: Tx<Item = Self::Event> + Send,
    {
        tx.send(ResolveStepEvent::Started)
            .await
            .map_err(|_| ResolveStepError::EventChannelClosed)?;

        let agent_name = input
            .manifest
            .agent
            .keys()
            .next()
            .cloned()
            .ok_or_else(|| ManifestError::resolution("manifest must define an agent"))?;
        let archetype_name = input
            .manifest
            .build
            .archetype()
            .to_string();
        let graph = &input
            .manifest
            .agent
            .get(&agent_name)
            .ok_or_else(|| ManifestError::resolution("manifest must define an agent"))?
            .graph;
        let graph_name = graph.graph().to_string();
        let graph_config = graph.config();

        let (context, archetype_config) = input
            .manifest
            .resolve(&*self.loader, &input.assets)
            .await?;

        let protocol_selections = context
            .http_server
            .as_ref()
            .map(|server| {
                server
                    .protocols
                    .iter()
                    .map(|protocol| (protocol.name().to_string(), protocol.config()))
                    .collect()
            })
            .unwrap_or_default();

        tx.send(ResolveStepEvent::Completed {
            agent_name: agent_name.clone(),
            archetype_name: archetype_name.clone(),
            tool_count: context.tools.len(),
            block_count: context.blocks.len(),
        })
        .await
        .map_err(|_| ResolveStepError::EventChannelClosed)?;

        Ok(ResolveStepOutput {
            agent_name,
            archetype_name,
            archetype_config,
            graph_name,
            graph_config,
            protocol_selections,
            context,
            assets: input.assets,
        })
    }
}
