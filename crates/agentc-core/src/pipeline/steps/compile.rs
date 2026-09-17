// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use std::path::PathBuf;
use thiserror::Error;

use agentc_compiler::{
    compiler::{CompileParams, OutputSink},
    generator::vfs::VirtualFileSystem,
    toolchain::{errors::ToolchainError, traits::ErasedToolchain},
    transformer::types::TransformedAsset,
};

use crate::pipeline::{
    sender::Tx,
    steps::{cleanup::CleanupStepInput, extract::ExtractStepOutput},
    traits::Step,
};

#[derive(Debug, Error)]
pub enum CompileStepError {
    #[error("toolchain error: {0}")]
    Toolchain(#[from] ToolchainError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("event channel closed")]
    EventChannelClosed,
}

pub enum CompileStepEvent {
    WritingFiles { project_dir: PathBuf },
    WriteCompleted { project_dir: PathBuf },
    Compiling { release: bool },
    CompilerStdout(String),
    CompilerStderr(String),
    CompileCompleted { output_dir: PathBuf },
}

pub struct CompileStepInput {
    pub agent_name: String,
    pub archetype_name: String,
    pub vfs: VirtualFileSystem,
    pub toolchain: Box<dyn ErasedToolchain>,
    pub assets: Vec<TransformedAsset>,
}

impl From<ExtractStepOutput> for CompileStepInput {
    fn from(output: ExtractStepOutput) -> Self {
        CompileStepInput {
            agent_name: output.agent_name,
            archetype_name: output.archetype_name,
            vfs: output.vfs,
            toolchain: output.toolchain,
            assets: output.assets,
        }
    }
}

pub struct CompileStepOutput {
    pub output_dir: PathBuf,
    pub toolchain: Box<dyn ErasedToolchain>,
    pub assets: Vec<TransformedAsset>,
}

impl From<CompileStepOutput> for CleanupStepInput<CompileStepOutput> {
    fn from(output: CompileStepOutput) -> Self {
        CleanupStepInput {
            assets: output.assets.clone(),
            inner: output,
        }
    }
}

struct CompilerOutputSink<'a, S>
where
    S: Tx<Item = CompileStepEvent> + Clone + Send + Sync + 'static,
{
    tx: &'a S,
}

#[async_trait]
impl<'a, S> OutputSink for CompilerOutputSink<'a, S>
where
    S: Tx<Item = CompileStepEvent> + Clone + Send + Sync + 'static,
{
    async fn stdout(&self, line: &str) {
        let _ = self
            .tx
            .send(CompileStepEvent::CompilerStdout(line.to_string()))
            .await;
    }

    async fn stderr(&self, line: &str) {
        let _ = self
            .tx
            .send(CompileStepEvent::CompilerStderr(line.to_string()))
            .await;
    }
}

pub struct CompileStep {
    project_dir: PathBuf,
    output_dir: PathBuf,
    cache_dir: Option<PathBuf>,
    release: bool,
    verbose: bool,
    args: Vec<String>,
}

impl CompileStep {
    pub fn new(
        project_dir: PathBuf,
        output_dir: PathBuf,
        cache_dir: Option<PathBuf>,
        release: bool,
        verbose: bool,
        args: Vec<String>,
    ) -> Self {
        Self {
            project_dir,
            output_dir,
            cache_dir,
            release,
            verbose,
            args,
        }
    }
}

#[async_trait]
impl Step for CompileStep {
    type Input = CompileStepInput;
    type Output = CompileStepOutput;
    type Event = CompileStepEvent;
    type Error = CompileStepError;

    async fn execute<S>(self, mut input: Self::Input, tx: S) -> Result<Self::Output, Self::Error>
    where
        S: Tx<Item = Self::Event> + Clone + Send + Sync + 'static,
    {
        tx.send(CompileStepEvent::WritingFiles { project_dir: self.project_dir.clone() })
            .await
            .map_err(|_| CompileStepError::EventChannelClosed)?;

        input
            .vfs
            .write_to(&self.project_dir)
            .await?;

        tx.send(CompileStepEvent::WriteCompleted { project_dir: self.project_dir.clone() })
            .await
            .map_err(|_| CompileStepError::EventChannelClosed)?;

        tx.send(CompileStepEvent::Compiling { release: self.release })
            .await
            .map_err(|_| CompileStepError::EventChannelClosed)?;

        input
            .toolchain
            .compile_erased(
                CompileParams::new(self.project_dir.clone(), self.output_dir.clone())
                    .maybe_with_cache_dir(self.cache_dir.clone())
                    .with_release(self.release)
                    .with_verbose(self.verbose)
                    .with_args(self.args.clone()),
                &CompilerOutputSink { tx: &tx },
            )
            .await?;

        tx.send(CompileStepEvent::CompileCompleted { output_dir: self.output_dir.clone() })
            .await
            .map_err(|_| CompileStepError::EventChannelClosed)?;

        Ok(CompileStepOutput {
            output_dir: self.output_dir,
            toolchain: input.toolchain,
            assets: input.assets,
        })
    }
}
