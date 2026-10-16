// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use rig_core::{
    client::ClientBuilder, http_client::ReqwestClient, markers::Missing, providers::openai,
};

use crate::{
    errors::ModelError,
    providers::{openai::model::OpenAiModel, rig::headers::Headers},
    traits::CompletionClient,
    types::{
        identity::{ModelId, ProviderId},
        inference::InferenceParams,
    },
};

#[derive(Clone)]
pub struct OpenAiClient {
    provider: ProviderId,
    inner: ClientBuilder<openai::OpenAICompletionsExtBuilder, Missing, ReqwestClient>,
    api_key: String,
    headers: Headers,
}

impl OpenAiClient {
    pub fn new(
        provider: ProviderId,
        client: ClientBuilder<openai::OpenAICompletionsExtBuilder, Missing, ReqwestClient>,
        api_key: String,
        headers: Headers,
    ) -> Self {
        Self {
            provider,
            inner: client,
            api_key,
            headers,
        }
    }
}

impl CompletionClient for OpenAiClient {
    type Model = OpenAiModel;

    fn provider(&self) -> ProviderId {
        self.provider.clone()
    }

    fn model(&self, model: ModelId, params: InferenceParams) -> Result<OpenAiModel, ModelError> {
        Ok(OpenAiModel::new(
            self.provider.clone(),
            self.inner.clone(),
            self.api_key.clone(),
            self.headers.clone(),
            model,
            params,
        ))
    }
}
