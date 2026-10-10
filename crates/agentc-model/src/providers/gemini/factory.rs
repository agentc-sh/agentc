// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;

use crate::{
    errors::ModelError,
    providers::gemini::{client::GeminiClient, config::GeminiConfig, constants::KIND},
    traits::ClientFactory,
    types::identity::{ProviderId, ProviderKind},
};

/// Factory for constructing [`GeminiClient`] instances from [`GeminiConfig`].
pub struct GeminiFactory;

#[async_trait]
impl ClientFactory for GeminiFactory {
    type Config = GeminiConfig;
    type Client = GeminiClient;

    fn kind() -> ProviderKind {
        KIND
    }

    async fn build(
        &self,
        provider: ProviderId,
        config: GeminiConfig,
    ) -> Result<GeminiClient, ModelError> {
        config.build_client(provider)
    }
}
