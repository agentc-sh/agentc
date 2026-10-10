// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;

use crate::{
    errors::ModelError,
    providers::ollama::{client::OllamaClient, config::OllamaConfig, constants::KIND},
    traits::ClientFactory,
    types::identity::{ProviderId, ProviderKind},
};

/// Factory for constructing [`OllamaClient`] instances from
/// [`OllamaConfig`]. Register with
/// [`ModelRegistry`](crate::registry::ModelRegistry) to enable dynamic
/// provider dispatch.
pub struct OllamaFactory;

#[async_trait]
impl ClientFactory for OllamaFactory {
    type Config = OllamaConfig;
    type Client = OllamaClient;

    fn kind() -> ProviderKind {
        KIND
    }

    async fn build(
        &self,
        provider: ProviderId,
        config: OllamaConfig,
    ) -> Result<OllamaClient, ModelError> {
        config.build_client(provider)
    }
}
