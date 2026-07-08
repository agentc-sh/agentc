// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use crate::{
    errors::ModelError,
    providers::gemini::{client::GeminiClient, config::GeminiConfig, constants::PROVIDER},
    traits::ClientFactory,
    types::identity::ProviderId,
};

/// Factory for constructing [`GeminiClient`] instances from [`GeminiConfig`].
pub struct GeminiFactory;

impl GeminiFactory {
    pub fn provider() -> ProviderId {
        PROVIDER.into()
    }
}

impl ClientFactory for GeminiFactory {
    type Config = GeminiConfig;
    type Client = GeminiClient;

    fn provider(&self) -> ProviderId {
        Self::provider()
    }

    fn build(&self, config: GeminiConfig) -> Result<GeminiClient, ModelError> {
        config.build_client()
    }
}
