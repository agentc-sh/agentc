// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use rig_core::providers::ollama;
use serde::{Deserialize, Serialize};

use crate::{errors::ModelError, providers::ollama::client::OllamaClient};

/// Configuration for constructing an [`OllamaClient`](crate::providers::ollama::OllamaClient).
///
/// If `base_url` is `None`, the client connects to the Ollama default of
/// `http://localhost:11434`.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct OllamaConfig {
    pub base_url: Option<String>,
}

impl OllamaConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = Some(base_url.into());
        self
    }

    /// Construct an [`OllamaClient`] from this config.
    pub fn build_client(&self) -> Result<OllamaClient, ModelError> {
        let mut builder = ollama::Client::builder().api_key(rig_core::client::Nothing);

        if let Some(url) = &self.base_url {
            builder = builder.base_url(url);
        }

        let inner = builder
            .build()
            .map_err(|e| ModelError::configuration(e.to_string()))?;

        Ok(OllamaClient::new(inner))
    }
}
