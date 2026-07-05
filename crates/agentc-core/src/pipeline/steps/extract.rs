// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use std::path::PathBuf;
use thiserror::Error;

use agentc_blocks::runtime::{EmbeddedAsset, RuntimeError};
use agentc_compiler::{
    asset::types::AssetOrigin,
    compiler::Compiler,
    generator::vfs::VirtualFileSystem,
    transformer::types::{AssetArtifact, TransformedAsset},
};

use crate::pipeline::{
    sender::Tx,
    steps::{cleanup::CleanupStepInput, generate::GenerateStepOutput},
    traits::Step,
};

#[derive(Debug, Error)]
pub enum ExtractStepError {
    #[error("runtime extraction error: {0}")]
    Runtime(#[from] RuntimeError),

    #[error("event channel closed")]
    EventChannelClosed,
}

pub enum ExtractStepEvent {
    Extracting { asset_count: usize },
    Extracted { runtime_dir: PathBuf },
}

pub struct ExtractStepInput {
    pub agent_name: String,
    pub archetype_name: String,
    pub vfs: VirtualFileSystem,
    pub compiler: Box<dyn Compiler>,
    pub assets: Vec<TransformedAsset>,
    pub embedded_assets: Vec<&'static EmbeddedAsset>,
}

impl From<GenerateStepOutput> for ExtractStepInput {
    fn from(output: GenerateStepOutput) -> Self {
        ExtractStepInput {
            agent_name: output.agent_name,
            archetype_name: output.archetype_name,
            vfs: output.vfs,
            compiler: output.compiler,
            assets: output.assets,
            embedded_assets: output.embedded_assets,
        }
    }
}

pub struct ExtractStepOutput {
    pub agent_name: String,
    pub archetype_name: String,
    pub vfs: VirtualFileSystem,
    pub compiler: Box<dyn Compiler>,
    pub assets: Vec<TransformedAsset>,
}

impl From<ExtractStepOutput> for CleanupStepInput<ExtractStepOutput> {
    fn from(output: ExtractStepOutput) -> Self {
        CleanupStepInput {
            assets: output.assets.clone(),
            inner: output,
        }
    }
}

pub struct ExtractStep {
    runtime_dir: PathBuf,
    ephemeral: bool,
}

impl ExtractStep {
    pub fn new(runtime_dir: PathBuf, ephemeral: bool) -> Self {
        Self { runtime_dir, ephemeral }
    }
}

#[async_trait]
impl Step for ExtractStep {
    type Input = ExtractStepInput;
    type Output = ExtractStepOutput;
    type Event = ExtractStepEvent;
    type Error = ExtractStepError;

    async fn execute<S>(self, mut input: Self::Input, tx: S) -> Result<Self::Output, Self::Error>
    where
        S: Tx<Item = Self::Event> + Clone + Send + Sync + 'static,
    {
        tx.send(ExtractStepEvent::Extracting { asset_count: input.embedded_assets.len() })
            .await
            .map_err(|_| ExtractStepError::EventChannelClosed)?;

        agentc_blocks::runtime::extract_all(&input.embedded_assets, self.runtime_dir.clone())
            .await?;

        tx.send(ExtractStepEvent::Extracted { runtime_dir: self.runtime_dir.clone() })
            .await
            .map_err(|_| ExtractStepError::EventChannelClosed)?;

        if self.ephemeral {
            input.assets.push(TransformedAsset {
                uri: "embedded::runtime".to_string(),
                origin: AssetOrigin::Internal,
                artifacts: vec![AssetArtifact::ephemeral_path(
                    "runtime_dir",
                    self.runtime_dir,
                )],
            });
        }

        Ok(ExtractStepOutput {
            agent_name: input.agent_name,
            archetype_name: input.archetype_name,
            vfs: input.vfs,
            compiler: input.compiler,
            assets: input.assets,
        })
    }
}
