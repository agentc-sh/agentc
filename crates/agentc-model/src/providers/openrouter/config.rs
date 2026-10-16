// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use rig_core::{http_client::ReqwestClient, providers::openrouter};
use serde::{Deserialize, Serialize};
use std::env;

use crate::{
    errors::ModelError,
    providers::{
        openrouter::{client::OpenRouterClient, constants::API_KEY_ENV},
        rig::headers::{HeaderSource, Headers},
    },
    types::identity::ProviderId,
};

/// Configuration for constructing an [`OpenRouterClient`].
///
/// If `api_key` is `None`, reads from the `OPENROUTER_API_KEY` environment variable.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct OpenRouterConfig {
    pub api_key: Option<String>,
    #[serde(default)]
    pub headers: Headers,
}

impl OpenRouterConfig {
    pub fn with_header(mut self, name: impl Into<String>, source: HeaderSource) -> Self {
        self.headers = self.headers.with(name, source);
        self
    }

    pub fn build_client(&self, provider: ProviderId) -> Result<OpenRouterClient, ModelError> {
        Ok(OpenRouterClient::new(
            provider.clone(),
            openrouter::Client::builder().http_client(ReqwestClient::default()),
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
