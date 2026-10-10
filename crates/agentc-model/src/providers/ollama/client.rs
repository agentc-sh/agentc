// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use rig_core::providers::ollama;

use crate::{
    errors::ModelError,
    providers::ollama::model::OllamaModel,
    traits::CompletionClient,
    types::{
        identity::{ModelId, ProviderId},
        inference::InferenceParams,
    },
};

#[derive(Clone)]
pub struct OllamaClient {
    provider: ProviderId,
    inner: ollama::Client,
}

impl OllamaClient {
    pub fn new(provider: ProviderId, client: ollama::Client) -> Self {
        Self { provider, inner: client }
    }
}

impl CompletionClient for OllamaClient {
    type Model = OllamaModel;

    fn provider(&self) -> ProviderId {
        self.provider.clone()
    }

    fn model(&self, model: ModelId, params: InferenceParams) -> Result<OllamaModel, ModelError> {
        Ok(OllamaModel::new(self.provider.clone(), self.inner.clone(), model, params))
    }
}
