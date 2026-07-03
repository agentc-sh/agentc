// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use rig::providers::openai;
use serde::{Deserialize, Serialize};
use std::env;

use crate::{errors::ModelError, providers::openai::client::OpenAiClient};

/// Configuration for constructing an [`OpenAiClient`](crate::providers::openai::OpenAiClient).
///
/// If `api_key` is `None`, the client will attempt to read the key from
/// the `OPENAI_API_KEY` environment variable.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiConfig {
    pub base_url: Option<String>,
    pub api_key: Option<String>,
}

impl OpenAiConfig {
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

    /// Construct an [`OpenAiClient`] from this config.
    pub fn build_client(&self) -> Result<OpenAiClient, ModelError> {
        Ok(OpenAiClient::new(
            openai::CompletionsClient::builder()
                .api_key(match &self.api_key {
                    Some(key) => key.clone(),
                    None => env::var("OPENAI_API_KEY")
                        .expect("OPENAI_API_KEY environment variable must be set if api_key is not provided"),
                })
                .build()
                .map_err(|e| ModelError::configuration(e.to_string()))?
        ))
    }
}
