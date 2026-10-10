// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;

use crate::{
    errors::ModelError,
    providers::xai::{client::XaiClient, config::XaiConfig, constants::KIND},
    traits::ClientFactory,
    types::identity::{ProviderId, ProviderKind},
};

/// Factory for constructing [`XaiClient`] instances from [`XaiConfig`].
pub struct XaiFactory;

#[async_trait]
impl ClientFactory for XaiFactory {
    type Config = XaiConfig;
    type Client = XaiClient;

    fn kind() -> ProviderKind {
        KIND
    }

    async fn build(
        &self,
        provider: ProviderId,
        config: XaiConfig,
    ) -> Result<XaiClient, ModelError> {
        config.build_client(provider)
    }
}
