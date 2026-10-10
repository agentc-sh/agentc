// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use rig_core::providers::xai;

use crate::{
    errors::ModelError,
    providers::xai::model::XaiModel,
    traits::CompletionClient,
    types::{
        identity::{ModelId, ProviderId},
        inference::InferenceParams,
    },
};

#[derive(Clone)]
pub struct XaiClient {
    provider: ProviderId,
    inner: xai::Client,
}

impl XaiClient {
    pub fn new(provider: ProviderId, client: xai::Client) -> Self {
        Self { provider, inner: client }
    }
}

impl CompletionClient for XaiClient {
    type Model = XaiModel;

    fn provider(&self) -> ProviderId {
        self.provider.clone()
    }

    fn model(&self, model: ModelId, params: InferenceParams) -> Result<XaiModel, ModelError> {
        Ok(XaiModel::new(self.provider.clone(), self.inner.clone(), model, params))
    }
}
