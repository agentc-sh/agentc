// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use rig_core::{
    client::ClientBuilder, http_client::ReqwestClient, markers::Missing, providers::xai,
};

use crate::{
    errors::ModelError,
    providers::{rig::headers::Headers, xai::model::XaiModel},
    traits::CompletionClient,
    types::{
        identity::{ModelId, ProviderId},
        inference::InferenceParams,
    },
};

#[derive(Clone)]
pub struct XaiClient {
    provider: ProviderId,
    inner: ClientBuilder<xai::client::XAiExtBuilder, Missing, ReqwestClient>,
    api_key: String,
    headers: Headers,
}

impl XaiClient {
    pub fn new(
        provider: ProviderId,
        client: ClientBuilder<xai::client::XAiExtBuilder, Missing, ReqwestClient>,
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

impl CompletionClient for XaiClient {
    type Model = XaiModel;

    fn provider(&self) -> ProviderId {
        self.provider.clone()
    }

    fn model(&self, model: ModelId, params: InferenceParams) -> Result<XaiModel, ModelError> {
        Ok(XaiModel::new(
            self.provider.clone(),
            self.inner.clone(),
            self.api_key.clone(),
            self.headers.clone(),
            model,
            params,
        ))
    }
}
