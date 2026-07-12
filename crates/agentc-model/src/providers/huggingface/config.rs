// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use rig_core::providers::huggingface;
use serde::{Deserialize, Serialize};
use std::env;

use crate::{errors::ModelError, providers::huggingface::client::HuggingFaceClient};

/// Configuration for constructing a [`HuggingFaceClient`].
///
/// If `api_key` is `None`, reads from the `HUGGINGFACE_API_KEY` environment variable.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct HuggingFaceConfig {
    pub api_key: Option<String>,
    pub base_url: Option<String>,
}

impl HuggingFaceConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = Some(base_url.into());
        self
    }

    /// Construct a [`HuggingFaceClient`] from this config.
    pub fn build_client(&self) -> Result<HuggingFaceClient, ModelError> {
        let mut builder = huggingface::Client::builder().api_key(match &self.api_key {
            Some(key) => key.clone(),
            None => env::var("HUGGINGFACE_API_KEY")
                .expect("HUGGINGFACE_API_KEY must be set if api_key is not provided"),
        });

        if let Some(url) = &self.base_url {
            builder = builder.base_url(url);
        }

        Ok(HuggingFaceClient::new(
            builder
                .build()
                .map_err(|e| ModelError::configuration(e.to_string()))?,
        ))
    }
}
