// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use crate::{
    errors::ModelError,
    providers::anthropic::{client::AnthropicClient, config::AnthropicConfig, constants::PROVIDER},
    traits::ClientFactory,
    types::identity::ProviderId,
};

/// Factory for constructing [`AnthropicClient`] instances from
/// [`AnthropicConfig`]. Register with
/// [`ModelRegistry`](crate::registry::ModelRegistry) to enable dynamic
/// provider dispatch.
pub struct AnthropicFactory;

impl AnthropicFactory {
    pub fn provider() -> ProviderId {
        PROVIDER.into()
    }
}

impl ClientFactory for AnthropicFactory {
    type Config = AnthropicConfig;
    type Client = AnthropicClient;

    fn provider(&self) -> ProviderId {
        Self::provider()
    }

    fn build(&self, config: AnthropicConfig) -> Result<AnthropicClient, ModelError> {
        config.build_client()
    }
}
