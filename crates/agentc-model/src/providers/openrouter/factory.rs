// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;

use crate::{
    errors::ModelError,
    providers::openrouter::{client::OpenRouterClient, config::OpenRouterConfig, constants::KIND},
    traits::ClientFactory,
    types::identity::{ProviderId, ProviderKind},
};

/// Factory for constructing [`OpenRouterClient`] instances from [`OpenRouterConfig`].
pub struct OpenRouterFactory;

#[async_trait]
impl ClientFactory for OpenRouterFactory {
    type Config = OpenRouterConfig;
    type Client = OpenRouterClient;

    fn kind() -> ProviderKind {
        KIND
    }

    async fn build(
        &self,
        provider: ProviderId,
        config: OpenRouterConfig,
    ) -> Result<OpenRouterClient, ModelError> {
        config.build_client(provider)
    }
}
