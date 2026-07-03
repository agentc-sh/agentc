// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use thiserror::Error;

use agentc_compiler::asset::{AssetError, AssetResolver, ResolvedAsset};

use crate::{
    manifest::Manifest,
    pipeline::{sender::Tx, traits::Step},
};

#[derive(Debug, Error)]
pub enum FetchStepError {
    #[error("asset error: {0}")]
    Asset(#[from] AssetError),

    #[error("event channel closed")]
    EventChannelClosed,
}

pub enum FetchStepEvent {
    Started,
    Completed { count: usize },
}

pub struct FetchStepInput {
    pub manifest: Manifest,
}

pub struct FetchStepOutput {
    pub manifest: Manifest,
    pub assets: Vec<ResolvedAsset>,
}

pub struct FetchStep {
    resolver: AssetResolver,
}

impl FetchStep {
    pub fn new(resolver: AssetResolver) -> Self {
        Self { resolver }
    }
}

#[async_trait]
impl Step for FetchStep {
    type Input = FetchStepInput;
    type Output = FetchStepOutput;
    type Event = FetchStepEvent;
    type Error = FetchStepError;

    async fn execute<S>(self, input: Self::Input, tx: S) -> Result<Self::Output, Self::Error>
    where
        S: Tx<Item = Self::Event> + Send,
    {
        tx.send(FetchStepEvent::Started)
            .await
            .map_err(|_| FetchStepError::EventChannelClosed)?;

        let assets = self
            .resolver
            .resolve_all(&input.manifest.collect_assets())
            .await?;

        tx.send(FetchStepEvent::Completed { count: assets.len() })
            .await
            .map_err(|_| FetchStepError::EventChannelClosed)?;

        Ok(FetchStepOutput { manifest: input.manifest, assets })
    }
}
