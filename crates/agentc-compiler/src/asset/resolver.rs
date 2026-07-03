// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use crate::asset::{
    errors::AssetError,
    handler::AssetHandler,
    store::ArtifactStore,
    types::{AssetRef, ResolvedAsset},
};

/// Resolves asset URIs to local paths, fetching remote artifacts as needed.
///
/// Handlers are tried in registration order until the first handler that returns
/// `true` from [`AssetHandler::can_handle`](crate::resolver::handler::AssetHandler::can_handle). A built-in
/// [`LocalFileHandler`] is registered by default.
pub struct AssetResolver {
    handlers: Vec<Box<dyn AssetHandler>>,
    store: ArtifactStore,
}

impl AssetResolver {
    /// Create a new [`AssetResolver`](crate::resolver::resolver::AssetResolver) with the given artifact store.
    ///
    /// Registers [`LocalFileHandler`](crate::resolver::handler::LocalFileHandler) by default.
    pub fn new(handlers: Vec<Box<dyn AssetHandler>>, store: ArtifactStore) -> Self {
        Self { handlers, store }
    }

    /// Create a new [`AssetResolverBuilder`](crate::resolver::resolver::AssetResolverBuilder) for constructing a
    /// [`AssetResolver`](crate::resolver::resolver::AssetResolver) with custom handlers and/or store.
    pub fn builder() -> AssetResolverBuilder {
        AssetResolverBuilder::new()
    }

    /// Register an additional handler. Handlers are tried in registration
    /// order after the built-in [`LocalFileHandler`](crate::resolver::handler::LocalFileHandler).
    pub fn register<T>(&mut self, handler: T)
    where
        T: AssetHandler + 'static,
    {
        self.handlers.push(Box::new(handler));
    }

    /// Register an additional handler already boxed. Handlers are tried in registration
    /// order after the built-in [`LocalFileHandler`](crate::resolver::handler::LocalFileHandler).
    pub fn register_boxed(&mut self, handler: Box<dyn AssetHandler>) {
        self.handlers.push(handler);
    }

    /// Resolve a single [`AssetRef`](crate::resolver::handler::AssetRef) to a registry entry, fetching if necessary.
    pub async fn resolve(&self, asset_ref: &AssetRef) -> Result<ResolvedAsset, AssetError> {
        let uri = &asset_ref.uri;
        let dest = self.store.path_for(uri);

        if self.store.is_cached(uri) {
            return Ok(ResolvedAsset {
                local_path: dest,
                uri: uri.clone(),
                origin: asset_ref.origin.clone(),
            });
        }

        self.handlers
            .iter()
            .find(|h| h.can_handle(uri))
            .ok_or_else(|| AssetError::no_handler(uri))?
            .fetch(uri, &dest)
            .await?;

        Ok(ResolvedAsset {
            local_path: dest,
            uri: uri.clone(),
            origin: asset_ref.origin.clone(),
        })
    }

    /// Resolve all [`AssetRef`](crate::asset::handler::AssetRef)s, deduplicating the fetch work by URI
    /// while emitting one [`ResolvedAsset`] per input ref.
    ///
    /// When multiple refs share the same URI (e.g. several JS tools backed by the same bundle),
    /// the source is fetched only once. Every ref still produces its own [`ResolvedAsset`] entry
    /// carrying its distinct `origin`, so downstream steps can match each tool by name.
    pub async fn resolve_all(&self, assets: &[AssetRef]) -> Result<Vec<ResolvedAsset>, AssetError> {
        let mut resolved = Vec::with_capacity(assets.len());

        for asset_ref in assets {
            resolved.push(self.resolve(asset_ref).await?);
        }

        Ok(resolved)
    }
}

/// Builder for constructing a [`AssetResolver`](crate::resolver::resolver::AssetResolver) with custom handlers and/or store.
pub struct AssetResolverBuilder {
    handlers: Vec<Box<dyn AssetHandler>>,
    store: Option<ArtifactStore>,
}

impl AssetResolverBuilder {
    /// Create a new [`AssetResolverBuilder`](crate::resolver::resolver::AssetResolverBuilder) with no handlers or store.
    pub fn new() -> Self {
        Self { handlers: Vec::new(), store: None }
    }

    /// Set the artifact store to use for the resolver. This is required before calling `build()`.
    pub fn with_store(mut self, store: ArtifactStore) -> Self {
        self.store = Some(store);
        self
    }

    /// Add a handler to the resolver. Handlers are tried in registration order.
    pub fn with_handler<T>(mut self, handler: T) -> Self
    where
        T: AssetHandler + 'static,
    {
        self.handlers.push(Box::new(handler));
        self
    }

    /// Add a boxed handler to the resolver. Handlers are tried in registration order.
    pub fn with_boxed_handler(mut self, handler: Box<dyn AssetHandler>) -> Self {
        self.handlers.push(handler);
        self
    }

    /// Build the [`AssetResolver`](crate::resolver::resolver::AssetResolver) with the configured handlers and store.
    pub fn build(self) -> AssetResolver {
        AssetResolver::new(
            self.handlers,
            self.store
                .expect("artifact store is required"),
        )
    }
}

impl Default for AssetResolverBuilder {
    fn default() -> Self {
        Self::new()
    }
}
