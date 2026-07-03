// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::collections::HashMap;

use crate::{
    asset::types::ResolvedAsset,
    transformer::{
        errors::TransformError,
        traits::{AssetTransformer, TransformSink},
        types::{AssetArtifact, TransformedAsset},
    },
};

/// A registry of [`AssetTransformer`](crate::transformer::traits::AssetTransformer)s
/// applied in registration order.
///
/// For each fetched asset, all transformers that return `true` from
/// `can_transform` are run, and their outputs are collected into a single
/// [`TransformedAsset`](crate::transformer::types::TransformedAsset). Assets with no matching transformer are passed
/// through with a single `"raw"` artifact pointing to the original local path.
pub struct TransformerRegistry {
    transformers: Vec<Box<dyn AssetTransformer>>,
}

impl TransformerRegistry {
    pub fn new() -> Self {
        Self { transformers: Vec::new() }
    }

    pub fn register<T>(&mut self, transformer: T)
    where
        T: AssetTransformer + 'static,
    {
        self.transformers
            .push(Box::new(transformer));
    }

    pub fn with_transformer<T>(mut self, transformer: T) -> Self
    where
        T: AssetTransformer + 'static,
    {
        self.register(transformer);
        self
    }

    /// Process a single resolved asset through all matching transformers,
    /// collecting all their outputs into a [`TransformedAsset`](crate::transformer::types::TransformedAsset).
    ///
    /// If no transformer matches, the asset is passed through as a `"raw"` artifact.
    pub async fn process(
        &self,
        asset: &ResolvedAsset,
        sink: &dyn TransformSink,
    ) -> Result<TransformedAsset, TransformError> {
        let mut artifacts = Vec::new();

        for transformer in &self.transformers {
            if transformer
                .can_transform(&asset.local_path, &asset.origin)
                .await
            {
                artifacts.append(
                    &mut transformer
                        .transform(&asset.local_path, &asset.origin, sink)
                        .await?,
                );
            }
        }

        if artifacts.is_empty() {
            artifacts.push(AssetArtifact::path("raw", asset.local_path.clone()));
        }

        Ok(TransformedAsset {
            uri: asset.uri.clone(),
            origin: asset.origin.clone(),
            artifacts,
        })
    }

    /// Process all fetched assets, running each unique URI through the transformer only once.
    ///
    /// When multiple assets share the same URI (e.g. several JS tools backed by the same bundle),
    /// the transform runs only once for that URI. Every input asset still produces its own
    /// [`TransformedAsset`] entry carrying its distinct `origin`, so downstream steps can match
    /// each tool by name.
    pub async fn process_all(
        &self,
        assets: &[ResolvedAsset],
        sink: &dyn TransformSink,
    ) -> Result<Vec<TransformedAsset>, TransformError> {
        let mut by_uri = HashMap::<&str, TransformedAsset>::new();

        for asset in assets {
            if !by_uri.contains_key(asset.uri.as_str()) {
                by_uri.insert(&asset.uri, self.process(asset, sink).await?);
            }
        }

        assets
            .iter()
            .map(|asset| {
                Ok(TransformedAsset {
                    origin: asset.origin.clone(),
                    ..by_uri[asset.uri.as_str()].clone()
                })
            })
            .collect()
    }
}

impl Default for TransformerRegistry {
    fn default() -> Self {
        let mut reg = Self::new();

        #[cfg(feature = "javascript")]
        reg.register(crate::transformer::javascript::JavascriptTransformer::new());

        #[cfg(feature = "python")]
        reg.register(crate::transformer::python::PythonTransformer::new());

        reg.register(crate::transformer::skill::SkillTransformer::new());

        reg
    }
}
