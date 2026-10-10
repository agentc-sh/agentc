// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use rig_core::providers::openrouter;

use crate::{
    errors::ModelError,
    providers::openrouter::model::OpenRouterModel,
    traits::CompletionClient,
    types::{
        identity::{ModelId, ProviderId},
        inference::InferenceParams,
    },
};

#[derive(Clone)]
pub struct OpenRouterClient {
    provider: ProviderId,
    inner: openrouter::Client,
}

impl OpenRouterClient {
    pub fn new(provider: ProviderId, client: openrouter::Client) -> Self {
        Self { provider, inner: client }
    }
}

impl CompletionClient for OpenRouterClient {
    type Model = OpenRouterModel;

    fn provider(&self) -> ProviderId {
        self.provider.clone()
    }

    fn model(
        &self,
        model: ModelId,
        params: InferenceParams,
    ) -> Result<OpenRouterModel, ModelError> {
        Ok(OpenRouterModel::new(self.provider.clone(), self.inner.clone(), model, params))
    }
}
