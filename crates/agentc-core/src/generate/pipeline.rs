// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::{path::PathBuf, sync::Arc};
use tokio::sync::mpsc::{self, error::SendError};

use agentc_blocks::{archetype::resolver::ArchetypeResolver, context::ResolvedContext};
use agentc_compiler::{
    asset::AssetResolver,
    generator::{blocks::traits::Block, loader::ResourceLoader},
    transformer::TransformerRegistry,
};

use crate::{
    generate::{
        errors::GenerateError,
        types::{GenerateEvent, GenerateResult},
    },
    manifest::Manifest,
    pipeline::{
        steps::{
            cleanup::CleanupStep,
            extract::{ExtractStep, ExtractStepOutput},
            fetch::{FetchStep, FetchStepInput},
            generate::GenerateStep,
            resolve::ResolveStep,
            transform::TransformStep,
        },
        traits::Pipeline,
    },
};

impl From<SendError<GenerateEvent>> for GenerateError {
    fn from(_: SendError<GenerateEvent>) -> Self {
        GenerateError::event_channel_closed()
    }
}

pub struct GeneratePipeline {
    manifest: Manifest,
    asset_resolver: AssetResolver,
    loader: Arc<dyn ResourceLoader>,
    archetype_resolver: Arc<ArchetypeResolver>,
    transformer_registry: TransformerRegistry,
    blocks: Vec<Box<dyn Block<ResolvedContext>>>,
    runtime_dir: PathBuf,
    cleanup: bool,
    tx: mpsc::Sender<GenerateEvent>,
}

impl GeneratePipeline {
    pub fn builder() -> GeneratePipelineBuilder {
        GeneratePipelineBuilder::new()
    }

    pub async fn run(self) -> Result<GenerateResult, GenerateError> {
        let _ = self
            .tx
            .send(GenerateEvent::GenerateStarted { agent_name: self.manifest.agent_name()? })
            .await;

        let result = Pipeline::<_, _, _, _, GenerateError>::new()
            .step(FetchStep::new(self.asset_resolver))
            .step(TransformStep::new(self.transformer_registry))
            .step(ResolveStep::new(self.loader))
            .step(GenerateStep::new(self.archetype_resolver, self.blocks))
            .step(ExtractStep::new(self.runtime_dir, false))
            .step(CleanupStep::<ExtractStepOutput>::new(!self.cleanup))
            .run(FetchStepInput { manifest: self.manifest }, self.tx.clone())
            .await
            .map(|output| GenerateResult {
                agent_name: output.inner.agent_name,
                archetype_name: output.inner.archetype_name,
                vfs: output.inner.vfs,
            });

        if let Err(ref e) = result {
            let _ = self
                .tx
                .send(GenerateEvent::Failure { error: e.to_string() })
                .await;
        }

        result
    }
}

pub struct GeneratePipelineBuilder {
    manifest: Option<Manifest>,
    asset_resolver: Option<AssetResolver>,
    loader: Option<Arc<dyn ResourceLoader>>,
    archetype_resolver: Option<Arc<ArchetypeResolver>>,
    transformer_registry: TransformerRegistry,
    blocks: Vec<Box<dyn Block<ResolvedContext>>>,
    runtime_dir: Option<PathBuf>,
    cleanup: bool,
    stream_capacity: usize,
}

impl Default for GeneratePipelineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl GeneratePipelineBuilder {
    pub fn new() -> Self {
        Self {
            manifest: None,
            asset_resolver: None,
            loader: None,
            archetype_resolver: None,
            transformer_registry: TransformerRegistry::new(),
            blocks: Vec::new(),
            runtime_dir: None,
            cleanup: false,
            stream_capacity: 256,
        }
    }

    pub fn runtime_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.runtime_dir = Some(dir.into());
        self
    }

    pub fn manifest(mut self, manifest: impl Into<Manifest>) -> Self {
        self.manifest = Some(manifest.into());
        self
    }

    pub fn asset_resolver(mut self, resolver: AssetResolver) -> Self {
        self.asset_resolver = Some(resolver);
        self
    }

    pub fn loader<R>(mut self, loader: R) -> Self
    where
        R: ResourceLoader + 'static,
    {
        self.loader = Some(Arc::new(loader));
        self
    }

    pub fn loader_arc(mut self, loader: Arc<dyn ResourceLoader>) -> Self {
        self.loader = Some(loader);
        self
    }

    pub fn archetype_resolver(mut self, resolver: ArchetypeResolver) -> Self {
        self.archetype_resolver = Some(Arc::new(resolver));
        self
    }

    pub fn archetype_resolver_arc(mut self, resolver: Arc<ArchetypeResolver>) -> Self {
        self.archetype_resolver = Some(resolver);
        self
    }

    pub fn transformer_registry(mut self, registry: TransformerRegistry) -> Self {
        self.transformer_registry = registry;
        self
    }

    pub fn block<B>(mut self, block: B) -> Self
    where
        B: Block<ResolvedContext> + 'static,
    {
        self.blocks.push(Box::new(block));
        self
    }

    pub fn block_boxed(mut self, block: Box<dyn Block<ResolvedContext>>) -> Self {
        self.blocks.push(block);
        self
    }

    pub fn blocks(
        mut self,
        blocks: impl IntoIterator<Item = Box<dyn Block<ResolvedContext>>>,
    ) -> Self {
        self.blocks.extend(blocks);
        self
    }

    pub fn cleanup(mut self, cleanup: bool) -> Self {
        self.cleanup = cleanup;
        self
    }

    pub fn stream_capacity(mut self, capacity: impl Into<usize>) -> Self {
        self.stream_capacity = capacity.into();
        self
    }

    pub fn build(self) -> Result<(GeneratePipeline, mpsc::Receiver<GenerateEvent>), GenerateError> {
        let (tx, rx) = mpsc::channel(self.stream_capacity);

        Ok((
            GeneratePipeline {
                manifest: self
                    .manifest
                    .ok_or_else(|| GenerateError::pipeline_configuration("manifest is required"))?,
                asset_resolver: self.asset_resolver.ok_or_else(|| {
                    GenerateError::pipeline_configuration("asset resolver is required")
                })?,
                loader: self.loader.ok_or_else(|| {
                    GenerateError::pipeline_configuration("resource loader is required")
                })?,
                archetype_resolver: self.archetype_resolver.ok_or_else(|| {
                    GenerateError::pipeline_configuration("archetype resolver is required")
                })?,
                transformer_registry: self.transformer_registry,
                blocks: self.blocks,
                runtime_dir: self.runtime_dir.ok_or_else(|| {
                    GenerateError::pipeline_configuration("runtime_dir is required")
                })?,
                cleanup: self.cleanup,
                tx,
            },
            rx,
        ))
    }
}
