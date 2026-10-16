// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use rig_core::{
    client::ClientBuilder, http_client::ReqwestClient, markers::Missing, providers::anthropic,
};

use crate::{
    errors::ModelError,
    providers::{anthropic::model::AnthropicModel, rig::headers::Headers},
    traits::CompletionClient,
    types::{
        identity::{ModelId, ProviderId},
        inference::InferenceParams,
    },
};

#[derive(Clone)]
pub struct AnthropicClient {
    provider: ProviderId,
    inner: ClientBuilder<anthropic::client::AnthropicBuilder, Missing, ReqwestClient>,
    api_key: String,
    headers: Headers,
}

impl AnthropicClient {
    pub fn new(
        provider: ProviderId,
        client: ClientBuilder<anthropic::client::AnthropicBuilder, Missing, ReqwestClient>,
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

impl CompletionClient for AnthropicClient {
    type Model = AnthropicModel;

    fn provider(&self) -> ProviderId {
        self.provider.clone()
    }

    fn model(&self, model: ModelId, params: InferenceParams) -> Result<AnthropicModel, ModelError> {
        Ok(AnthropicModel::new(
            self.provider.clone(),
            self.inner.clone(),
            self.api_key.clone(),
            self.headers.clone(),
            model,
            params,
        ))
    }
}
