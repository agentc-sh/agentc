// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use rig_core::providers::anthropic;
use serde::{Deserialize, Serialize};
use std::env;

use crate::{
    errors::ModelError,
    providers::anthropic::{client::AnthropicClient, constants::API_KEY_ENV},
    types::identity::ProviderId,
};

/// Configuration for constructing an [`AnthropicClient`](crate::providers::anthropic::AnthropicClient).
///
/// If `api_key` is `None`, the client will attempt to read the key from
/// the `ANTHROPIC_API_KEY` environment variable.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicConfig {
    pub base_url: Option<String>,
    pub api_key: Option<String>,
}

impl AnthropicConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = Some(base_url.into());
        self
    }

    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    /// Construct an [`AnthropicClient`] from this config.
    pub fn build_client(&self, provider: ProviderId) -> Result<AnthropicClient, ModelError> {
        let mut builder = anthropic::Client::builder().api_key(
            match &self.api_key {
                Some(key) => key.clone(),
                None => env::var(API_KEY_ENV).map_err(|_| {
                    ModelError::configuration(format!(
                        "provider '{provider}' has no api_key and {API_KEY_ENV} is not set"
                    ))
                })?,
            },
        );

        if let Some(url) = &self.base_url {
            builder = builder.base_url(url);
        }

        Ok(
            AnthropicClient::new(
                provider,
                builder
                    .build()
                    .map_err(|e| ModelError::configuration(e.to_string()))?,
            )
        )
    }
}
