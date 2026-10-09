// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use rig_core::providers::openai;

use crate::{
    errors::ModelError,
    providers::openai::model::OpenAiModel,
    traits::CompletionClient,
    types::{
        identity::{ModelId, ProviderId},
        inference::InferenceParams,
    },
};

#[derive(Clone)]
pub struct OpenAiClient {
    provider: ProviderId,
    inner: openai::CompletionsClient,
}

impl OpenAiClient {
    pub fn new(provider: ProviderId, client: openai::CompletionsClient) -> Self {
        Self { provider, inner: client }
    }
}

impl CompletionClient for OpenAiClient {
    type Model = OpenAiModel;

    fn provider(&self) -> ProviderId {
        self.provider.clone()
    }

    fn model(&self, model: ModelId, params: InferenceParams) -> Result<OpenAiModel, ModelError> {
        Ok(OpenAiModel::new(self.provider.clone(), self.inner.clone(), model, params))
    }
}
