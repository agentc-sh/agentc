// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use crate::{
    errors::ModelError,
    providers::huggingface::{
        client::HuggingFaceClient, config::HuggingFaceConfig, constants::PROVIDER,
    },
    traits::ClientFactory,
    types::identity::ProviderId,
};

/// Factory for constructing [`HuggingFaceClient`] instances from [`HuggingFaceConfig`].
pub struct HuggingFaceFactory;

impl HuggingFaceFactory {
    pub fn provider() -> ProviderId {
        PROVIDER.into()
    }
}

impl ClientFactory for HuggingFaceFactory {
    type Config = HuggingFaceConfig;
    type Client = HuggingFaceClient;

    fn provider(&self) -> ProviderId {
        Self::provider()
    }

    fn build(&self, config: HuggingFaceConfig) -> Result<HuggingFaceClient, ModelError> {
        config.build_client()
    }
}
