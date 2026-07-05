// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use rig_core::providers::ollama;

use crate::{
    providers::ollama::{constants::PROVIDER, model::OllamaModel},
    traits::CompletionClient,
    types::{
        identity::{ModelId, ProviderId},
        inference::InferenceParams,
    },
};

#[derive(Clone)]
pub struct OllamaClient {
    inner: ollama::Client,
}

impl OllamaClient {
    pub fn new(client: ollama::Client) -> Self {
        Self { inner: client }
    }
}

impl CompletionClient for OllamaClient {
    type Model = OllamaModel;

    fn provider(&self) -> ProviderId {
        PROVIDER.into()
    }

    fn model(&self, model: ModelId, params: InferenceParams) -> OllamaModel {
        OllamaModel::new(self.inner.clone(), model, params)
    }
}
