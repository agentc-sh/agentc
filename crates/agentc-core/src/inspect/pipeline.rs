// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::sync::Arc;
use tokio::sync::mpsc::{self, error::SendError};

use agentc_blocks::catalog::CompilationCatalog;
use agentc_compiler::{
    asset::AssetResolver, generator::loader::ResourceLoader, transformer::TransformerRegistry,
};

use crate::{
    inspect::{
        errors::InspectError,
        types::{InspectEvent, InspectResult},
    },
    manifest::Manifest,
    pipeline::{
        steps::{
            compose::ComposeStep,
            fetch::{FetchStep, FetchStepInput},
            resolve::ResolveStep,
            transform::TransformStep,
        },
        traits::Pipeline,
    },
};

impl From<SendError<InspectEvent>> for InspectError {
    fn from(_: SendError<InspectEvent>) -> Self {
        InspectError::event_channel_closed()
    }
}

pub struct InspectPipeline {
    manifest: Manifest,
    asset_resolver: AssetResolver,
    loader: Arc<dyn ResourceLoader>,
    catalog: Arc<CompilationCatalog>,
    transformer_registry: TransformerRegistry,
    tx: mpsc::Sender<InspectEvent>,
}

impl InspectPipeline {
    pub fn builder() -> InspectPipelineBuilder {
        InspectPipelineBuilder::new()
    }

    pub async fn run(self) -> Result<InspectResult, InspectError> {
        let result = Pipeline::<_, _, _, _, InspectError>::new()
            .step(FetchStep::new(self.asset_resolver))
            .step(TransformStep::new(self.transformer_registry))
            .step(ResolveStep::new(self.loader))
            .step(ComposeStep::new(self.catalog, Vec::new()))
            .run(FetchStepInput { manifest: self.manifest }, self.tx.clone())
            .await
            .map(|output| InspectResult {
                agent_name: output.agent_name,
                archetype_name: output.archetype_name,
                graph_name: output.graph_name,
                protocol_names: output.protocol_names,
                context: output.context,
            });

        if let Err(ref e) = result {
            let _ = self
                .tx
                .send(InspectEvent::Failure { error: e.to_string() })
                .await;
        }

        result
    }
}

pub struct InspectPipelineBuilder {
    manifest: Option<Manifest>,
    asset_resolver: Option<AssetResolver>,
    loader: Option<Arc<dyn ResourceLoader>>,
    catalog: Option<Arc<CompilationCatalog>>,
    transformer_registry: TransformerRegistry,
    stream_capacity: usize,
}

impl Default for InspectPipelineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl InspectPipelineBuilder {
    pub fn new() -> Self {
        Self {
            manifest: None,
            asset_resolver: None,
            loader: None,
            catalog: None,
            transformer_registry: TransformerRegistry::new(),
            stream_capacity: 256,
        }
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

    pub fn stream_capacity(mut self, capacity: impl Into<usize>) -> Self {
        self.stream_capacity = capacity.into();
        self
    }

    pub fn build(self) -> Result<(InspectPipeline, mpsc::Receiver<InspectEvent>), InspectError> {
        let (tx, rx) = mpsc::channel(self.stream_capacity);

        Ok((
            InspectPipeline {
                manifest: self
                    .manifest
                    .ok_or_else(|| InspectError::pipeline_configuration("manifest is required"))?,
                asset_resolver: self.asset_resolver.ok_or_else(|| {
                    InspectError::pipeline_configuration("asset resolver is required")
                })?,
                loader: self.loader.ok_or_else(|| {
                    InspectError::pipeline_configuration("resource loader is required")
                })?,
                catalog: self.catalog.ok_or_else(|| {
                    InspectError::pipeline_configuration("compilation catalog is required")
                })?,
                transformer_registry: self.transformer_registry,
                tx,
            },
            rx,
        ))
    }
}
