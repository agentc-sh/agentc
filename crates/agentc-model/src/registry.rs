// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::{
    collections::{HashMap, HashSet},
    fmt::{Debug, Formatter, Result as FmtResult},
    sync::Arc,
};

use serde::Serialize;
use serde_json::{Value, to_value};

use crate::{
    errors::ModelError,
    traits::{ClientFactory, CompletionModel, ErasedClientFactory, ErasedCompletionClient},
    types::{
        identity::{ModelId, ProviderId},
        inference::InferenceParams,
    },
};

/// A registry for model providers and their clients.
#[derive(Clone)]
pub struct ModelRegistry {
    factories: HashMap<ProviderId, Arc<dyn ErasedClientFactory>>,
    configs: HashMap<ProviderId, Value>,
    constraints: HashMap<ProviderId, Option<HashSet<ModelId>>>,
    provider_params: HashMap<ProviderId, InferenceParams>,
    model_params: HashMap<(ProviderId, ModelId), InferenceParams>,
}

impl ModelRegistry {
    /// Create a new, empty [`ModelRegistry`](crate::registry::ModelRegistry).
    pub fn new() -> Self {
        Self {
            factories: HashMap::new(),
            configs: HashMap::new(),
            constraints: HashMap::new(),
            provider_params: HashMap::new(),
            model_params: HashMap::new(),
        }
    }

    /// Get a builder for creating a new [`ModelRegistry`](crate::registry::ModelRegistry) with registered providers and configs.
    pub fn builder() -> ModelRegistryBuilder {
        ModelRegistryBuilder::new()
    }

    fn build_client<C>(
        &self,
        provider: &ProviderId,
        config: C,
    ) -> Result<Arc<dyn ErasedCompletionClient>, ModelError>
    where
        C: Serialize,
    {
        self.factories
            .get(provider)
            .ok_or_else(|| ModelError::unknown_provider(provider.clone()))?
            .build_erased(to_value(config).map_err(ModelError::Serialization)?)
    }

    /// Register a new client factory for a provider. This will overwrite any existing factory for the same provider.
    pub fn register_factory<T>(&mut self, factory: T)
    where
        T: ClientFactory + 'static,
    {
        self.factories
            .insert(factory.provider().clone(), Arc::new(factory));
    }

    /// Register a default config for a provider. This can be used by clients that want to support default configurations for providers.
    pub fn register_config<C>(
        &mut self,
        provider: impl Into<ProviderId>,
        config: C,
    ) -> Result<(), ModelError>
    where
        C: Serialize,
    {
        match to_value(config) {
            Ok(value) => {
                self.configs
                    .insert(provider.into(), value);
            }
            Err(e) => return Err(e.into()),
        }

        Ok(())
    }

    /// Register a set of allowed models for a provider.
    pub fn register_constraints<P, M, I>(&mut self, provider: P, models: M)
    where
        P: Into<ProviderId>,
        M: IntoIterator<Item = I>,
        I: Into<ModelId>,
    {
        self.constraints.insert(
            provider.into(),
            Some(
                models
                    .into_iter()
                    .map(Into::into)
                    .collect(),
            ),
        );
    }

    /// Register provider-level [`InferenceParams`] defaults for a provider. These are applied to
    /// every model from this provider unless overridden by model-specific params.
    pub fn register_provider_params(
        &mut self,
        provider: impl Into<ProviderId>,
        params: InferenceParams,
    ) {
        self.provider_params
            .insert(provider.into(), params);
    }

    /// Register model-specific [`InferenceParams`] for a particular model within a provider.
    /// These are merged on top of provider-level params; model-level values win.
    pub fn register_model_params(
        &mut self,
        provider: impl Into<ProviderId>,
        model: impl Into<ModelId>,
        params: InferenceParams,
    ) {
        self.model_params
            .insert((provider.into(), model.into()), params);
    }

    /// Get a builder for creating clients for a specific provider.
    pub fn provider(&self, provider: impl Into<ProviderId>) -> ModelClientBuilder<'_> {
        let provider = provider.into();
        ModelClientBuilder {
            registry: self,
            provider: provider.clone(),
            config: self.configs.get(&provider).cloned(),
        }
    }
}

/// A builder for creating clients and models for a specific provider.
pub struct ModelClientBuilder<'a> {
    registry: &'a ModelRegistry,
    provider: ProviderId,
    config: Option<Value>,
}

impl<'a> ModelClientBuilder<'a> {
    /// Build a client for this provider with the given config.
    pub fn build<C>(&self, config: C) -> Result<Arc<dyn ErasedCompletionClient>, ModelError>
    where
        C: Serialize,
    {
        self.registry
            .build_client(&self.provider, config)
    }

    /// Build and immediately select a model from this provider with the given config and model name.
    pub fn model_with_config<C>(
        &self,
        config: C,
        model: impl Into<ModelId>,
    ) -> Result<Arc<dyn CompletionModel>, ModelError>
    where
        C: Serialize,
    {
        let model = model.into();
        Ok(self.build(config)?.model_erased(
            model.clone(),
            self.registry
                .provider_params
                .get(&self.provider)
                .cloned()
                .unwrap_or_default()
                .merge(
                    self.registry
                        .model_params
                        .get(&(self.provider.clone(), model))
                        .cloned()
                        .unwrap_or_default(),
                ),
        ))
    }

    /// Build and immediately select a model from this provider with the default config (if registered) and the given model name.
    pub fn model(&self, model: impl Into<ModelId>) -> Result<Arc<dyn CompletionModel>, ModelError> {
        let model = model.into();

        if let Some(allowed) = self
            .registry
            .constraints
            .get(&self.provider)
            .and_then(|opt| opt.as_ref())
            && !allowed.contains(&model)
        {
            return Err(ModelError::model_not_allowed(self.provider.clone(), model));
        }

        self.model_with_config(
            self.config.clone().ok_or_else(|| {
                ModelError::configuration(format!(
                    "no default configuration for provider '{}'",
                    self.provider
                ))
            })?,
            model,
        )
    }
}

impl Default for ModelRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for ModelRegistry {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.debug_struct("ModelRegistry")
            .field(
                "providers",
                &self
                    .factories
                    .keys()
                    .collect::<Vec<_>>(),
            )
            .finish()
    }
}

#[derive(Clone)]
pub struct ModelRegistryBuilder {
    factories: HashMap<ProviderId, Arc<dyn ErasedClientFactory>>,
    configs: HashMap<ProviderId, Value>,
    constraints: HashMap<ProviderId, Option<HashSet<ModelId>>>,
    provider_params: HashMap<ProviderId, InferenceParams>,
    model_params: HashMap<(ProviderId, ModelId), InferenceParams>,
}

impl ModelRegistryBuilder {
    pub fn new() -> Self {
        Self {
            factories: HashMap::new(),
            configs: HashMap::new(),
            constraints: HashMap::new(),
            provider_params: HashMap::new(),
            model_params: HashMap::new(),
        }
    }

    pub fn register_factory<T>(&mut self, factory: T) -> &mut Self
    where
        T: ClientFactory + 'static,
    {
        self.factories
            .insert(factory.provider().clone(), Arc::new(factory));
        self
    }

    pub fn with_factory<T>(mut self, factory: T) -> Self
    where
        T: ClientFactory + 'static,
    {
        self.register_factory(factory);
        self
    }

    pub fn register_config<C>(
        &mut self,
        provider: impl Into<ProviderId>,
        config: C,
    ) -> Result<&mut Self, ModelError>
    where
        C: Serialize,
    {
        match to_value(config) {
            Ok(value) => {
                self.configs
                    .insert(provider.into(), value);
            }
            Err(e) => return Err(e.into()),
        }

        Ok(self)
    }

    pub fn with_config<C>(
        mut self,
        provider: impl Into<ProviderId>,
        config: C,
    ) -> Result<Self, ModelError>
    where
        C: Serialize,
    {
        self.register_config(provider, config)?;
        Ok(self)
    }

    pub fn register_constraints<P, M, I>(&mut self, provider: P, models: M) -> &mut Self
    where
        P: Into<ProviderId>,
        M: IntoIterator<Item = I>,
        I: Into<ModelId>,
    {
        self.constraints.insert(
            provider.into(),
            Some(
                models
                    .into_iter()
                    .map(Into::into)
                    .collect(),
            ),
        );
        self
    }

    pub fn with_constraints<P, M, I>(mut self, provider: P, models: M) -> Self
    where
        P: Into<ProviderId>,
        M: IntoIterator<Item = I>,
        I: Into<ModelId>,
    {
        self.register_constraints(provider, models);
        self
    }

    pub fn register_provider_params(
        &mut self,
        provider: impl Into<ProviderId>,
        params: InferenceParams,
    ) -> &mut Self {
        self.provider_params
            .insert(provider.into(), params);
        self
    }

    pub fn with_provider_params(
        mut self,
        provider: impl Into<ProviderId>,
        params: InferenceParams,
    ) -> Self {
        self.register_provider_params(provider, params);
        self
    }

    pub fn register_model_params(
        &mut self,
        provider: impl Into<ProviderId>,
        model: impl Into<ModelId>,
        params: InferenceParams,
    ) -> &mut Self {
        self.model_params
            .insert((provider.into(), model.into()), params);
        self
    }

    pub fn with_model_params(
        mut self,
        provider: impl Into<ProviderId>,
        model: impl Into<ModelId>,
        params: InferenceParams,
    ) -> Self {
        self.register_model_params(provider, model, params);
        self
    }

    pub fn build(&self) -> ModelRegistry {
        ModelRegistry {
            factories: self.factories.clone(),
            configs: self.configs.clone(),
            constraints: self.constraints.clone(),
            provider_params: self.provider_params.clone(),
            model_params: self.model_params.clone(),
        }
    }
}

impl Default for ModelRegistryBuilder {
    fn default() -> Self {
        Self::new()
    }
}
