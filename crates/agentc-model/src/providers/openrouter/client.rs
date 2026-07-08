// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use rig_core::providers::openrouter;

use crate::{
    providers::openrouter::{constants::PROVIDER, model::OpenRouterModel},
    traits::CompletionClient,
    types::{
        identity::{ModelId, ProviderId},
        inference::InferenceParams,
    },
};

#[derive(Clone)]
pub struct OpenRouterClient {
    inner: openrouter::Client,
}

impl OpenRouterClient {
    pub fn new(client: openrouter::Client) -> Self {
        Self { inner: client }
    }
}

impl CompletionClient for OpenRouterClient {
    type Model = OpenRouterModel;

    fn provider(&self) -> ProviderId {
        PROVIDER.into()
    }

    fn model(&self, model: ModelId, params: InferenceParams) -> OpenRouterModel {
        OpenRouterModel::new(self.inner.clone(), model, params)
    }
}
