// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;

use crate::{
    errors::ModelError,
    providers::openai::{client::OpenAiClient, config::OpenAiConfig, constants::KIND},
    traits::ClientFactory,
    types::identity::{ProviderId, ProviderKind},
};

/// Factory for constructing [`OpenAiClient`] instances from
/// [`OpenAiConfig`]. Register with
/// [`ModelRegistry`](crate::registry::ModelRegistry) to enable dynamic
/// provider dispatch.
pub struct OpenAiFactory;

#[async_trait]
impl ClientFactory for OpenAiFactory {
    type Config = OpenAiConfig;
    type Client = OpenAiClient;

    fn kind() -> ProviderKind {
        KIND
    }

    async fn build(
        &self,
        provider: ProviderId,
        config: OpenAiConfig,
    ) -> Result<OpenAiClient, ModelError> {
        config.build_client(provider)
    }
}
