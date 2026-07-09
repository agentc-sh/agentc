// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use rig_core::providers::huggingface;

use crate::{
    providers::huggingface::{constants::PROVIDER, model::HuggingFaceModel},
    traits::CompletionClient,
    types::{
        identity::{ModelId, ProviderId},
        inference::InferenceParams,
    },
};

#[derive(Clone)]
pub struct HuggingFaceClient {
    inner: huggingface::Client,
}

impl HuggingFaceClient {
    pub fn new(client: huggingface::Client) -> Self {
        Self { inner: client }
    }
}

impl CompletionClient for HuggingFaceClient {
    type Model = HuggingFaceModel;

    fn provider(&self) -> ProviderId {
        PROVIDER.into()
    }

    fn model(&self, model: ModelId, params: InferenceParams) -> HuggingFaceModel {
        HuggingFaceModel::new(self.inner.clone(), model, params)
    }
}
