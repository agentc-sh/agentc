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
        identity::{ModelId, ProviderId, ProviderKind},
        inference::InferenceParams,
    },
};

#[derive(Clone)]
struct RegisteredProvider {
    client: Arc<dyn ErasedCompletionClient>,
    constraints: Option<HashSet<ModelId>>,
    params: InferenceParams,
    model_params: HashMap<ModelId, InferenceParams>,
}

impl RegisteredProvider {
    fn model(&self, model: ModelId) -> Result<Arc<dyn CompletionModel>, ModelError> {
        if let Some(allowed) = &self.constraints
            && !allowed.contains(&model)
        {
            return Err(ModelError::model_not_allowed(self.client.provider(), model));
        }

        self.client.model_erased(
            model.clone(),
            self.params
                .clone()
                .merge(
                    self.model_params
                        .get(&model)
                        .cloned()
                        .unwrap_or_default(),
                ),
        )
    }
}

/// A registry for model providers and their clients.
#[derive(Clone)]
pub struct ModelRegistry {
    providers: HashMap<ProviderId, RegisteredProvider>,
}

impl ModelRegistry {
    /// Create a new, empty [`ModelRegistry`](crate::registry::ModelRegistry).
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
        }
    }

    /// Get a builder for creating a new [`ModelRegistry`](crate::registry::ModelRegistry) with registered providers and configs.
    pub fn builder() -> ModelRegistryBuilder {
        ModelRegistryBuilder::new()
    }

    /// Get a builder for selecting models from a provider instance.
    pub fn provider(&self, provider: impl Into<ProviderId>) -> ModelClientBuilder<'_> {
        ModelClientBuilder {
            registry: self,
            provider: provider.into(),
        }
    }
}

/// A builder for selecting models from a provider instance.
pub struct ModelClientBuilder<'a> {
    registry: &'a ModelRegistry,
    provider: ProviderId,
}

impl<'a> ModelClientBuilder<'a> {
    /// Select a model from this provider instance.
    pub fn model(&self, model: impl Into<ModelId>) -> Result<Arc<dyn CompletionModel>, ModelError> {
        self.registry
            .providers
            .get(&self.provider)
            .ok_or_else(|| ModelError::unknown_provider(self.provider.clone()))?
            .model(model.into())
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
                    .providers
                    .keys()
                    .collect::<Vec<_>>(),
            )
            .finish()
    }
}

#[derive(Clone)]
struct ProviderDeclaration {
    kind: ProviderKind,
    config: Value,
}

#[derive(Clone)]
pub struct ModelRegistryBuilder {
    factories: HashMap<ProviderKind, Arc<dyn ErasedClientFactory>>,
    providers: HashMap<ProviderId, ProviderDeclaration>,
    constraints: HashMap<ProviderId, HashSet<ModelId>>,
    provider_params: HashMap<ProviderId, InferenceParams>,
    model_params: HashMap<ProviderId, HashMap<ModelId, InferenceParams>>,
}

impl ModelRegistryBuilder {
    pub fn new() -> Self {
        Self {
            factories: HashMap::new(),
            providers: HashMap::new(),
            constraints: HashMap::new(),
            provider_params: HashMap::new(),
            model_params: HashMap::new(),
        }
    }

    pub fn register_factory<F>(&mut self, factory: F) -> &mut Self
    where
        F: ClientFactory + 'static,
    {
        self.factories
            .insert(F::kind(), Arc::new(factory));
        self
    }

    pub fn with_factory<F>(mut self, factory: F) -> Self
    where
        F: ClientFactory + 'static,
    {
        self.register_factory(factory);
        self
    }

    pub fn register_provider<F>(
        &mut self,
        provider: impl Into<ProviderId>,
        config: F::Config,
    ) -> Result<&mut Self, ModelError>
    where
        F: ClientFactory,
        F::Config: Serialize,
    {
        self.providers.insert(
            provider.into(),
            ProviderDeclaration {
                kind: F::kind(),
                config: to_value(config)?,
            },
        );

        Ok(self)
    }

    pub fn with_provider<F>(
        mut self,
        provider: impl Into<ProviderId>,
        config: F::Config,
    ) -> Result<Self, ModelError>
    where
        F: ClientFactory,
        F::Config: Serialize,
    {
        self.register_provider::<F>(provider, config)?;
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
            models
                .into_iter()
                .map(Into::into)
                .collect(),
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
            .entry(provider.into())
            .or_default()
            .insert(model.into(), params);
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

    pub async fn build(mut self) -> Result<ModelRegistry, ModelError> {
        if let Some(provider) = self
            .constraints
            .keys()
            .chain(self.provider_params.keys())
            .chain(self.model_params.keys())
            .find(|provider| !self.providers.contains_key(*provider))
        {
            return Err(ModelError::unknown_provider(provider.clone()));
        }

        let mut providers = HashMap::new();

        for (id, declaration) in self.providers {
            let client = self
                .factories
                .get(&declaration.kind)
                .ok_or_else(|| ModelError::unknown_provider(id.clone()))?
                .build_erased(id.clone(), declaration.config)
                .await?;

            providers.insert(
                id.clone(),
                RegisteredProvider {
                    client,
                    constraints: self.constraints.remove(&id),
                    params: self
                        .provider_params
                        .remove(&id)
                        .unwrap_or_default(),
                    model_params: self
                        .model_params
                        .remove(&id)
                        .unwrap_or_default(),
                },
            );
        }

        Ok(ModelRegistry { providers })
    }
}

impl Default for ModelRegistryBuilder {
    fn default() -> Self {
        Self::new()
    }
}
