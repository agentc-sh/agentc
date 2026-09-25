// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::sync::Arc;
use tokio::sync::mpsc::{self, error::SendError};

use agentc_blocks::{catalog::CompilationCatalog, context::ResolvedContext};
use agentc_compiler::{
    asset::AssetResolver,
    generator::{blocks::traits::Block, loader::ResourceLoader},
    transformer::TransformerRegistry,
};

use crate::{
    manifest::Manifest,
    pipeline::{
        steps::{
            cleanup::CleanupStep,
            compile::{CompileStep, CompileStepOutput},
            compose::ComposeStep,
            extract::ExtractStep,
            fetch::{FetchStep, FetchStepInput},
            generate::GenerateStep,
            launch::LaunchStep,
            preflight::PreflightStep,
            resolve::ResolveStep,
            transform::TransformStep,
        },
        traits::Pipeline,
    },
    run::{
        errors::RunError,
        preconditions::RunSupported,
        types::{RunEvent, RunParams, RunResult},
    },
};

impl From<SendError<RunEvent>> for RunError {
    fn from(_: SendError<RunEvent>) -> Self {
        RunError::event_channel_closed()
    }
}

pub struct RunPipeline {
    manifest: Manifest,
    params: RunParams,
    asset_resolver: AssetResolver,
    loader: Arc<dyn ResourceLoader>,
    catalog: Arc<CompilationCatalog>,
    transformer_registry: TransformerRegistry,
    blocks: Vec<Box<dyn Block<ResolvedContext>>>,
    skip_cleanup: bool,
    tx: mpsc::Sender<RunEvent>,
}

impl RunPipeline {
    pub fn builder() -> RunPipelineBuilder {
        RunPipelineBuilder::new()
    }

    pub async fn run(self) -> Result<RunResult, RunError> {
        let _ = self
            .tx
            .send(RunEvent::RunStarted { agent_name: self.manifest.agent_name()? })
            .await;

        let result = Pipeline::<_, _, _, _, RunError>::new()
            .step(FetchStep::new(self.asset_resolver))
            .step(TransformStep::new(self.transformer_registry))
            .step(ResolveStep::new(self.loader))
            .step(ComposeStep::new(self.catalog, self.blocks))
            .step(PreflightStep::new().with(RunSupported))
            .step(GenerateStep::new())
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
                self.params.build_args,
            ))
            .step(CleanupStep::<CompileStepOutput>::new(self.skip_cleanup))
            .step(LaunchStep::new(self.params.context_dir, self.params.args))
            .run(FetchStepInput { manifest: self.manifest }, self.tx.clone())
            .await
            .map(|output| RunResult { exit_code: output.exit_code });

        if let Err(ref e) = result {
            let _ = self
                .tx
                .send(RunEvent::Failure { error: e.to_string() })
                .await;
        }

        result
    }
}

pub struct RunPipelineBuilder {
    manifest: Option<Manifest>,
    params: Option<RunParams>,
    asset_resolver: Option<AssetResolver>,
    loader: Option<Arc<dyn ResourceLoader>>,
    catalog: Option<Arc<CompilationCatalog>>,
    transformer_registry: TransformerRegistry,
    blocks: Vec<Box<dyn Block<ResolvedContext>>>,
    skip_cleanup: bool,
    stream_capacity: usize,
}

impl RunPipelineBuilder {
    pub fn new() -> Self {
        Self {
            manifest: None,
            params: None,
            asset_resolver: None,
            loader: None,
            catalog: None,
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

    pub fn params(mut self, params: RunParams) -> Self {
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

    pub fn catalog(mut self, catalog: CompilationCatalog) -> Self {
        self.catalog = Some(Arc::new(catalog));
        self
    }

    pub fn catalog_arc(mut self, catalog: Arc<CompilationCatalog>) -> Self {
        self.catalog = Some(catalog);
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

    pub fn build(self) -> Result<(RunPipeline, mpsc::Receiver<RunEvent>), RunError> {
        let (tx, rx) = mpsc::channel(self.stream_capacity);

        Ok((
            RunPipeline {
                manifest: self
                    .manifest
                    .ok_or_else(|| RunError::pipeline_configuration("manifest is required"))?,
                params: self
                    .params
                    .ok_or_else(|| RunError::pipeline_configuration("run params are required"))?,
                asset_resolver: self.asset_resolver.ok_or_else(|| {
                    RunError::pipeline_configuration("asset resolver is required")
                })?,
                loader: self.loader.ok_or_else(|| {
                    RunError::pipeline_configuration("resource loader is required")
                })?,
                catalog: self.catalog.ok_or_else(|| {
                    RunError::pipeline_configuration("compilation catalog is required")
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

impl Default for RunPipelineBuilder {
    fn default() -> Self {
        Self::new()
    }
}
