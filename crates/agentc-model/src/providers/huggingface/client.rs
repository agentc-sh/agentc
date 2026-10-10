// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use rig_core::providers::huggingface;

use crate::{
    errors::ModelError,
    providers::huggingface::model::HuggingFaceModel,
    traits::CompletionClient,
    types::{
        identity::{ModelId, ProviderId},
        inference::InferenceParams,
    },
};

#[derive(Clone)]
pub struct HuggingFaceClient {
    provider: ProviderId,
    inner: huggingface::Client,
}

impl HuggingFaceClient {
    pub fn new(provider: ProviderId, client: huggingface::Client) -> Self {
        Self { provider, inner: client }
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
        Ok(HuggingFaceModel::new(self.provider.clone(), self.inner.clone(), model, params))
    }
}
