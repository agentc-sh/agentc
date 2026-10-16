// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use rig_core::{
    client::ClientBuilder, http_client::ReqwestClient, markers::Missing, providers::gemini,
};

use crate::{
    errors::ModelError,
    providers::{gemini::model::GeminiModel, rig::headers::Headers},
    traits::CompletionClient,
    types::{
        identity::{ModelId, ProviderId},
        inference::InferenceParams,
    },
};

#[derive(Clone)]
pub struct GeminiClient {
    provider: ProviderId,
    inner: ClientBuilder<gemini::client::GeminiBuilder, Missing, ReqwestClient>,
    api_key: String,
    headers: Headers,
}

impl GeminiClient {
    pub fn new(
        provider: ProviderId,
        client: ClientBuilder<gemini::client::GeminiBuilder, Missing, ReqwestClient>,
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

impl CompletionClient for GeminiClient {
    type Model = GeminiModel;

    fn provider(&self) -> ProviderId {
        self.provider.clone()
    }

    fn model(&self, model: ModelId, params: InferenceParams) -> Result<GeminiModel, ModelError> {
        Ok(GeminiModel::new(
            self.provider.clone(),
            self.inner.clone(),
            self.api_key.clone(),
            self.headers.clone(),
            model,
            params,
        ))
    }
}
