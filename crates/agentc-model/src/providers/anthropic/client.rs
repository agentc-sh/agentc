// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use rig_core::providers::anthropic;

use crate::{
    providers::anthropic::{constants::PROVIDER, model::AnthropicModel},
    traits::CompletionClient,
    types::{
        identity::{ModelId, ProviderId},
        inference::InferenceParams,
    },
};

#[derive(Clone)]
pub struct AnthropicClient {
    inner: anthropic::Client,
}

impl AnthropicClient {
    pub fn new(client: anthropic::Client) -> Self {
        Self { inner: client }
    }
}

impl CompletionClient for AnthropicClient {
    type Model = AnthropicModel;

    fn provider(&self) -> ProviderId {
        PROVIDER.into()
    }

    fn model(&self, model: ModelId, params: InferenceParams) -> AnthropicModel {
        AnthropicModel::new(self.inner.clone(), model, params)
    }
}
