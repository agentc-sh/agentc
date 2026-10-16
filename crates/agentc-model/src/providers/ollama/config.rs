// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use rig_core::{http_client::ReqwestClient, providers::ollama};
use serde::{Deserialize, Serialize};

use crate::{
    errors::ModelError,
    providers::{
        ollama::client::OllamaClient,
        rig::headers::{HeaderSource, Headers},
    },
    types::identity::ProviderId,
};

/// Configuration for constructing an [`OllamaClient`](crate::providers::ollama::OllamaClient).
///
/// If `base_url` is `None`, the client connects to the Ollama default of
/// `http://localhost:11434`.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct OllamaConfig {
    pub base_url: Option<String>,
    #[serde(default)]
    pub headers: Headers,
}

impl OllamaConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = Some(base_url.into());
        self
    }

    pub fn with_header(mut self, name: impl Into<String>, source: HeaderSource) -> Self {
        self.headers = self.headers.with(name, source);
        self
    }

    /// Construct an [`OllamaClient`] from this config.
    pub fn build_client(&self, provider: ProviderId) -> Result<OllamaClient, ModelError> {
        let mut builder = ollama::Client::builder().http_client(ReqwestClient::default());

        if let Some(url) = &self.base_url {
            builder = builder.base_url(url);
        }

        Ok(OllamaClient::new(provider, builder, self.headers.clone()))
    }
}
