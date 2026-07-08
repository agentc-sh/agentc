// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use rig_core::providers::xai;

use crate::{
    providers::xai::{constants::PROVIDER, model::XaiModel},
    traits::CompletionClient,
    types::{
        identity::{ModelId, ProviderId},
        inference::InferenceParams,
    },
};

#[derive(Clone)]
pub struct XaiClient {
    inner: xai::Client,
}

impl XaiClient {
    pub fn new(client: xai::Client) -> Self {
        Self { inner: client }
    }
}

impl CompletionClient for XaiClient {
    type Model = XaiModel;

    fn provider(&self) -> ProviderId {
        PROVIDER.into()
    }

    fn model(&self, model: ModelId, params: InferenceParams) -> XaiModel {
        XaiModel::new(self.inner.clone(), model, params)
    }
}
