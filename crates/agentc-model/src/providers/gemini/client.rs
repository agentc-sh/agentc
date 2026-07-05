// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use rig_core::providers::gemini;

use crate::{
    providers::gemini::{constants::PROVIDER, model::GeminiModel},
    traits::CompletionClient,
    types::{
        identity::{ModelId, ProviderId},
        inference::InferenceParams,
    },
};

#[derive(Clone)]
pub struct GeminiClient {
    inner: gemini::Client,
}

impl GeminiClient {
    pub fn new(client: gemini::Client) -> Self {
        Self { inner: client }
    }
}

impl CompletionClient for GeminiClient {
    type Model = GeminiModel;

    fn provider(&self) -> ProviderId {
        PROVIDER.into()
    }

    fn model(&self, model: ModelId, params: InferenceParams) -> GeminiModel {
        GeminiModel::new(self.inner.clone(), model, params)
    }
}
