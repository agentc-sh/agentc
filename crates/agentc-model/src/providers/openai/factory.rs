// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use crate::{
    errors::ModelError,
    providers::openai::{client::OpenAiClient, config::OpenAiConfig, constants::PROVIDER},
    traits::ClientFactory,
    types::identity::ProviderId,
};

/// Factory for constructing [`OpenAiClient`] instances from
/// [`OpenAiConfig`]. Register with
/// [`ModelRegistry`](crate::registry::ModelRegistry) to enable dynamic
/// provider dispatch.
pub struct OpenAiFactory;

impl OpenAiFactory {
    pub fn provider() -> ProviderId {
        PROVIDER.into()
    }
}

impl ClientFactory for OpenAiFactory {
    type Config = OpenAiConfig;
    type Client = OpenAiClient;

    fn provider(&self) -> ProviderId {
        Self::provider()
    }

    fn build(&self, config: OpenAiConfig) -> Result<OpenAiClient, ModelError> {
        config.build_client()
    }
}
