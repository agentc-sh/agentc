// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;

use crate::{
    errors::ModelError,
    providers::huggingface::{
        client::HuggingFaceClient, config::HuggingFaceConfig, constants::KIND,
    },
    traits::ClientFactory,
    types::identity::{ProviderId, ProviderKind},
};

/// Factory for constructing [`HuggingFaceClient`] instances from [`HuggingFaceConfig`].
pub struct HuggingFaceFactory;

#[async_trait]
impl ClientFactory for HuggingFaceFactory {
    type Config = HuggingFaceConfig;
    type Client = HuggingFaceClient;

    fn kind() -> ProviderKind {
        KIND
    }

    async fn build(
        &self,
        provider: ProviderId,
        config: HuggingFaceConfig,
    ) -> Result<HuggingFaceClient, ModelError> {
        config.build_client(provider)
    }
}
