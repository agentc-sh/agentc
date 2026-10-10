// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use rig_core::providers::anthropic;

use crate::{
    errors::ModelError,
    providers::anthropic::model::AnthropicModel,
    traits::CompletionClient,
    types::{
        identity::{ModelId, ProviderId},
        inference::InferenceParams,
    },
};

#[derive(Clone)]
pub struct AnthropicClient {
    provider: ProviderId,
    inner: anthropic::Client,
}

impl AnthropicClient {
    pub fn new(provider: ProviderId, client: anthropic::Client) -> Self {
        Self { provider, inner: client }
    }
}

impl CompletionClient for AnthropicClient {
    type Model = AnthropicModel;

    fn provider(&self) -> ProviderId {
        self.provider.clone()
    }

    fn model(&self, model: ModelId, params: InferenceParams) -> Result<AnthropicModel, ModelError> {
        Ok(AnthropicModel::new(self.provider.clone(), self.inner.clone(), model, params))
    }
}
