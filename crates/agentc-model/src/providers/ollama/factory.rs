// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use crate::{
    errors::ModelError,
    providers::ollama::{client::OllamaClient, config::OllamaConfig, constants::PROVIDER},
    traits::ClientFactory,
    types::identity::ProviderId,
};

/// Factory for constructing [`OllamaClient`] instances from
/// [`OllamaConfig`]. Register with
/// [`ModelRegistry`](crate::registry::ModelRegistry) to enable dynamic
/// provider dispatch.
pub struct OllamaFactory;

impl OllamaFactory {
    pub fn provider() -> ProviderId {
        PROVIDER.into()
    }
}

impl ClientFactory for OllamaFactory {
    type Config = OllamaConfig;
    type Client = OllamaClient;

    fn provider(&self) -> ProviderId {
        Self::provider()
    }

    fn build(&self, config: OllamaConfig) -> Result<OllamaClient, ModelError> {
        config.build_client()
    }
}
