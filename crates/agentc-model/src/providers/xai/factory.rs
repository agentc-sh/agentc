// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use crate::{
    errors::ModelError,
    providers::xai::{client::XaiClient, config::XaiConfig, constants::PROVIDER},
    traits::ClientFactory,
    types::identity::ProviderId,
};

/// Factory for constructing [`XaiClient`] instances from [`XaiConfig`].
pub struct XaiFactory;

impl XaiFactory {
    pub fn provider() -> ProviderId {
        PROVIDER.into()
    }
}

impl ClientFactory for XaiFactory {
    type Config = XaiConfig;
    type Client = XaiClient;

    fn provider(&self) -> ProviderId {
        Self::provider()
    }

    fn build(&self, config: XaiConfig) -> Result<XaiClient, ModelError> {
        config.build_client()
    }
}
