// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use rig_core::{http_client::ReqwestClient, providers::openai};
use serde::{Deserialize, Serialize};
use std::env;

use crate::{
    errors::ModelError,
    providers::{
        openai::{client::OpenAiClient, constants::API_KEY_ENV},
        rig::headers::{HeaderSource, Headers},
    },
    types::identity::ProviderId,
};

/// Configuration for constructing an [`OpenAiClient`](crate::providers::openai::OpenAiClient).
///
/// If `api_key` is `None`, the client will attempt to read the key from
/// the `OPENAI_API_KEY` environment variable.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiConfig {
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    #[serde(default)]
    pub headers: Headers,
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

    pub fn with_header(mut self, name: impl Into<String>, source: HeaderSource) -> Self {
        self.headers = self.headers.with(name, source);
        self
    }

    /// Construct an [`OpenAiClient`] from this config.
    pub fn build_client(&self, provider: ProviderId) -> Result<OpenAiClient, ModelError> {
        let mut builder = openai::CompletionsClient::builder().http_client(ReqwestClient::default());

        if let Some(url) = &self.base_url {
            builder = builder.base_url(url);
        }

        Ok(OpenAiClient::new(
            provider.clone(),
            builder,
            match &self.api_key {
                Some(key) => key.clone(),
                None => env::var(API_KEY_ENV).map_err(|_| {
                    ModelError::configuration(format!(
                        "provider '{provider}' has no api_key and {API_KEY_ENV} is not set"
                    ))
                })?,
            },
            self.headers.clone(),
        ))
    }
}
