// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use rig_core::{
    client::ClientBuilder, http_client::ReqwestClient, markers::Missing, providers::huggingface,
};

use crate::{
    errors::ModelError,
    providers::{huggingface::model::HuggingFaceModel, rig::headers::Headers},
    traits::CompletionClient,
    types::{
        identity::{ModelId, ProviderId},
        inference::InferenceParams,
    },
};

#[derive(Clone)]
pub struct HuggingFaceClient {
    provider: ProviderId,
    inner: ClientBuilder<huggingface::client::HuggingFaceBuilder, Missing, ReqwestClient>,
    api_key: String,
    headers: Headers,
}

impl HuggingFaceClient {
    pub fn new(
        provider: ProviderId,
        client: ClientBuilder<huggingface::client::HuggingFaceBuilder, Missing, ReqwestClient>,
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

impl CompletionClient for HuggingFaceClient {
    type Model = HuggingFaceModel;

    fn provider(&self) -> ProviderId {
        self.provider.clone()
    }

    fn model(
        &self,
        model: ModelId,
        params: InferenceParams,
    ) -> Result<HuggingFaceModel, ModelError> {
        Ok(HuggingFaceModel::new(
            self.provider.clone(),
            self.inner.clone(),
            self.api_key.clone(),
            self.headers.clone(),
            model,
            params,
        ))
    }
}
