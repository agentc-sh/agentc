// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use rig_core::providers::gemini;

use crate::{
    errors::ModelError,
    providers::gemini::model::GeminiModel,
    traits::CompletionClient,
    types::{
        identity::{ModelId, ProviderId},
        inference::InferenceParams,
    },
};

#[derive(Clone)]
pub struct GeminiClient {
    provider: ProviderId,
    inner: gemini::Client,
}

impl GeminiClient {
    pub fn new(provider: ProviderId, client: gemini::Client) -> Self {
        Self { provider, inner: client }
    }
}

impl CompletionClient for GeminiClient {
    type Model = GeminiModel;

    fn provider(&self) -> ProviderId {
        self.provider.clone()
    }

    fn model(&self, model: ModelId, params: InferenceParams) -> Result<GeminiModel, ModelError> {
        Ok(GeminiModel::new(self.provider.clone(), self.inner.clone(), model, params))
    }
}
