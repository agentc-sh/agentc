// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use rig_core::providers::openai;

use crate::{
    providers::openai::{constants::PROVIDER, model::OpenAiModel},
    traits::CompletionClient,
    types::{
        identity::{ModelId, ProviderId},
        inference::InferenceParams,
    },
};

#[derive(Clone)]
pub struct OpenAiClient {
    inner: openai::CompletionsClient,
}

impl OpenAiClient {
    pub fn new(client: openai::CompletionsClient) -> Self {
        Self { inner: client }
    }
}

impl CompletionClient for OpenAiClient {
    type Model = OpenAiModel;

    fn provider(&self) -> ProviderId {
        PROVIDER.into()
    }

    fn model(&self, model: ModelId, params: InferenceParams) -> OpenAiModel {
        OpenAiModel::new(self.inner.clone(), model, params)
    }
}
