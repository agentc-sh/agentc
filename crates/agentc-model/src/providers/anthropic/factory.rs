// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;

use crate::{
    errors::ModelError,
    providers::anthropic::{client::AnthropicClient, config::AnthropicConfig, constants::KIND},
    traits::ClientFactory,
    types::identity::{ProviderId, ProviderKind},
};

/// Factory for constructing [`AnthropicClient`] instances from
/// [`AnthropicConfig`]. Register with
/// [`ModelRegistry`](crate::registry::ModelRegistry) to enable dynamic
/// provider dispatch.
pub struct AnthropicFactory;

#[async_trait]
impl ClientFactory for AnthropicFactory {
    type Config = AnthropicConfig;
    type Client = AnthropicClient;

    fn kind() -> ProviderKind {
        KIND
    }

    async fn build(
        &self,
        provider: ProviderId,
        config: AnthropicConfig,
    ) -> Result<AnthropicClient, ModelError> {
        config.build_client(provider)
    }
}
