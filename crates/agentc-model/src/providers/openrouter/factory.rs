// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use crate::{
    errors::ModelError,
    providers::openrouter::{
        client::OpenRouterClient,
        config::OpenRouterConfig,
        constants::PROVIDER,
    },
    traits::ClientFactory,
    types::identity::ProviderId,
};

/// Factory for constructing [`OpenRouterClient`] instances from [`OpenRouterConfig`].
pub struct OpenRouterFactory;

impl OpenRouterFactory {
    pub fn provider() -> ProviderId {
        PROVIDER.into()
    }
}

impl ClientFactory for OpenRouterFactory {
    type Config = OpenRouterConfig;
    type Client = OpenRouterClient;

    fn provider(&self) -> ProviderId {
        Self::provider()
    }

    fn build(&self, config: OpenRouterConfig) -> Result<OpenRouterClient, ModelError> {
        config.build_client()
    }
}
