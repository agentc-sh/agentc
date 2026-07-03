// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::sync::Arc;
use tokio::sync::mpsc::{self, error::SendError};

use agentc_blocks::{archetype::resolver::ArchetypeResolver, context::ResolvedContext};
use agentc_compiler::{
    asset::AssetResolver,
    generator::{blocks::traits::Block, loader::ResourceLoader},
    transformer::TransformerRegistry,
};

use crate::{
    build::{
        errors::BuildError,
        types::{BuildEvent, BuildParams, BuildResult},
    },
    manifest::Manifest,
    pipeline::{
        steps::{
            cleanup::CleanupStep,
            compile::{CompileStep, CompileStepOutput},
            extract::ExtractStep,
            fetch::{FetchStep, FetchStepInput},
            generate::GenerateStep,
            resolve::ResolveStep,
            transform::TransformStep,
        },
        traits::Pipeline,
    },
};

impl From<SendError<BuildEvent>> for BuildError {
    fn from(_: SendError<BuildEvent>) -> Self {
        BuildError::event_channel_closed()
    }
}

pub struct BuildPipeline {
    manifest: Manifest,
    params: BuildParams,
    asset_resolver: AssetResolver,
    loader: Arc<dyn ResourceLoader>,
    archetype_resolver: Arc<ArchetypeResolver>,
    transformer_registry: TransformerRegistry,
    blocks: Vec<Box<dyn Block<ResolvedContext>>>,
    skip_cleanup: bool,
    tx: mpsc::Sender<BuildEvent>,
}

impl BuildPipeline {
    pub fn builder() -> BuildPipelineBuilder {
        BuildPipelineBuilder::new()
    }

    pub async fn run(self) -> Result<BuildResult, BuildError> {
        let _ = self
            .tx
            .send(BuildEvent::BuildStarted { agent_name: self.manifest.agent_name()? })
            .await;

        let result = Pipeline::<_, _, _, _, BuildError>::new()
            .step(FetchStep::new(self.asset_resolver))
            .step(TransformStep::new(self.transformer_registry))
            .step(ResolveStep::new(self.loader))
            .step(GenerateStep::new(self.archetype_resolver, self.blocks))
            .step(ExtractStep::new(self.params.runtime_dir.clone(), true))
            .step(CompileStep::new(
                self.params.target_dir,
                self.params.output_dir,
                if self.params.no_cache {
                    None
                } else {
                    self.params.cache_dir
                },
                self.params.release,
                self.params.verbose,
                self.params.args,
            ))
            .step(CleanupStep::<CompileStepOutput>::new(self.skip_cleanup))
            .run(FetchStepInput { manifest: self.manifest }, self.tx.clone())
            .await
            .map(|output| BuildResult { artifact_dir: output.inner.output_dir });

        if let Err(ref e) = result {
            let _ = self
                .tx
                .send(BuildEvent::Failure { error: e.to_string() })
                .await;
        }

        result
    }
}

pub struct BuildPipelineBuilder {
    manifest: Option<Manifest>,
    params: Option<BuildParams>,
    asset_resolver: Option<AssetResolver>,
    loader: Option<Arc<dyn ResourceLoader>>,
    archetype_resolver: Option<Arc<ArchetypeResolver>>,
    transformer_registry: TransformerRegistry,
    blocks: Vec<Box<dyn Block<ResolvedContext>>>,
    skip_cleanup: bool,
    stream_capacity: usize,
}

impl BuildPipelineBuilder {
    pub fn new() -> Self {
        Self {
            manifest: None,
            params: None,
            asset_resolver: None,
            loader: None,
            archetype_resolver: None,
            transformer_registry: TransformerRegistry::new(),
            blocks: Vec::new(),
            skip_cleanup: false,
            stream_capacity: 256,
        }
    }

    pub fn manifest(mut self, manifest: impl Into<Manifest>) -> Self {
        self.manifest = Some(manifest.into());
        self
    }

    pub fn params(mut self, params: BuildParams) -> Self {
        self.params = Some(params);
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

    pub fn skip_cleanup(mut self, skip: bool) -> Self {
        self.skip_cleanup = skip;
        self
    }

    pub fn stream_capacity(mut self, capacity: impl Into<usize>) -> Self {
        self.stream_capacity = capacity.into();
        self
    }

    pub fn build(self) -> Result<(BuildPipeline, mpsc::Receiver<BuildEvent>), BuildError> {
        let (tx, rx) = mpsc::channel(self.stream_capacity);

        Ok((
            BuildPipeline {
                manifest: self
                    .manifest
                    .ok_or_else(|| BuildError::pipeline_configuration("manifest is required"))?,
                params: self.params.ok_or_else(|| {
                    BuildError::pipeline_configuration("build params are required")
                })?,
                asset_resolver: self.asset_resolver.ok_or_else(|| {
                    BuildError::pipeline_configuration("asset resolver is required")
                })?,
                loader: self.loader.ok_or_else(|| {
                    BuildError::pipeline_configuration("resource loader is required")
                })?,
                archetype_resolver: self.archetype_resolver.ok_or_else(|| {
                    BuildError::pipeline_configuration("archetype resolver is required")
                })?,
                transformer_registry: self.transformer_registry,
                blocks: self.blocks,
                skip_cleanup: self.skip_cleanup,
                tx,
            },
            rx,
        ))
    }
}

impl Default for BuildPipelineBuilder {
    fn default() -> Self {
        Self::new()
    }
}
