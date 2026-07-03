// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use thiserror::Error;

use agentc_compiler::{
    asset::types::ResolvedAsset,
    transformer::{TransformError, TransformSink, TransformerRegistry, types::TransformedAsset},
};

use crate::{
    manifest::Manifest,
    pipeline::{sender::Tx, steps::fetch::FetchStepOutput, traits::Step},
};

#[derive(Debug, Error)]
pub enum TransformStepError {
    #[error("transform error: {0}")]
    Transform(#[from] TransformError),

    #[error("event channel closed")]
    EventChannelClosed,
}

pub enum TransformStepEvent {
    Started { count: usize },
    TransformerStdout(String),
    TransformerStderr(String),
    Completed { count: usize },
}

struct TransformerOutputSink<'a, S>
where
    S: Tx<Item = TransformStepEvent> + Clone + Send + Sync + 'static,
{
    tx: &'a S,
}

#[async_trait]
impl<'a, S> TransformSink for TransformerOutputSink<'a, S>
where
    S: Tx<Item = TransformStepEvent> + Clone + Send + Sync + 'static,
{
    async fn stdout(&self, line: &str) {
        let _ = self
            .tx
            .send(TransformStepEvent::TransformerStdout(line.to_string()))
            .await;
    }

    async fn stderr(&self, line: &str) {
        let _ = self
            .tx
            .send(TransformStepEvent::TransformerStderr(line.to_string()))
            .await;
    }
}

pub struct TransformStepInput {
    pub manifest: Manifest,
    pub assets: Vec<ResolvedAsset>,
}

impl From<FetchStepOutput> for TransformStepInput {
    fn from(output: FetchStepOutput) -> Self {
        TransformStepInput {
            manifest: output.manifest,
            assets: output.assets,
        }
    }
}

pub struct TransformStepOutput {
    pub manifest: Manifest,
    pub assets: Vec<TransformedAsset>,
}

pub struct TransformStep {
    registry: TransformerRegistry,
}

impl TransformStep {
    pub fn new(registry: TransformerRegistry) -> Self {
        Self { registry }
    }
}

#[async_trait]
impl Step for TransformStep {
    type Input = TransformStepInput;
    type Output = TransformStepOutput;
    type Event = TransformStepEvent;
    type Error = TransformStepError;

    async fn execute<S>(self, input: Self::Input, tx: S) -> Result<Self::Output, Self::Error>
    where
        S: Tx<Item = Self::Event> + Clone + Send + Sync + 'static,
    {
        tx.send(TransformStepEvent::Started { count: input.assets.len() })
            .await
            .map_err(|_| TransformStepError::EventChannelClosed)?;

        let sink = TransformerOutputSink { tx: &tx };
        let assets = self
            .registry
            .process_all(&input.assets, &sink)
            .await?;

        tx.send(TransformStepEvent::Completed { count: assets.len() })
            .await
            .map_err(|_| TransformStepError::EventChannelClosed)?;

        Ok(TransformStepOutput { manifest: input.manifest, assets })
    }
}
