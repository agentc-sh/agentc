// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use rig_core::{
    client::ClientBuilder, http_client::ReqwestClient, markers::Missing, providers::ollama,
};

use crate::{
    errors::ModelError,
    providers::{ollama::model::OllamaModel, rig::headers::Headers},
    traits::CompletionClient,
    types::{
        identity::{ModelId, ProviderId},
        inference::InferenceParams,
    },
};

#[derive(Clone)]
pub struct OllamaClient {
    provider: ProviderId,
    inner: ClientBuilder<ollama::OllamaBuilder, Missing, ReqwestClient>,
    headers: Headers,
}

impl OllamaClient {
    pub fn new(
        provider: ProviderId,
        client: ClientBuilder<ollama::OllamaBuilder, Missing, ReqwestClient>,
        headers: Headers,
    ) -> Self {
        Self {
            provider,
            inner: client,
            headers,
        }
    }
}

impl CompletionClient for OllamaClient {
    type Model = OllamaModel;

    fn provider(&self) -> ProviderId {
        self.provider.clone()
    }

    fn model(&self, model: ModelId, params: InferenceParams) -> Result<OllamaModel, ModelError> {
        Ok(OllamaModel::new(
            self.provider.clone(),
            self.inner.clone(),
            self.headers.clone(),
            model,
            params,
        ))
    }
}
