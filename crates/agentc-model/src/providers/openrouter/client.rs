// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use rig_core::{
    client::ClientBuilder, http_client::ReqwestClient, markers::Missing, providers::openrouter,
};

use crate::{
    errors::ModelError,
    providers::{openrouter::model::OpenRouterModel, rig::headers::Headers},
    traits::CompletionClient,
    types::{
        identity::{ModelId, ProviderId},
        inference::InferenceParams,
    },
};

#[derive(Clone)]
pub struct OpenRouterClient {
    provider: ProviderId,
    inner: ClientBuilder<openrouter::OpenRouterExtBuilder, Missing, ReqwestClient>,
    api_key: String,
    headers: Headers,
}

impl OpenRouterClient {
    pub fn new(
        provider: ProviderId,
        client: ClientBuilder<openrouter::OpenRouterExtBuilder, Missing, ReqwestClient>,
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

impl CompletionClient for OpenRouterClient {
    type Model = OpenRouterModel;

    fn provider(&self) -> ProviderId {
        self.provider.clone()
    }

    fn model(
        &self,
        model: ModelId,
        params: InferenceParams,
    ) -> Result<OpenRouterModel, ModelError> {
        Ok(OpenRouterModel::new(
            self.provider.clone(),
            self.inner.clone(),
            self.api_key.clone(),
            self.headers.clone(),
            model,
            params,
        ))
    }
}
