// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use rig_core::providers::gemini;
use serde::{Deserialize, Serialize};
use std::env;

use crate::{
    errors::ModelError,
    providers::gemini::{client::GeminiClient, constants::API_KEY_ENV},
    types::identity::ProviderId,
};

/// Configuration for constructing a [`GeminiClient`].
///
/// If `api_key` is `None`, reads from the `GEMINI_API_KEY` environment variable.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct GeminiConfig {
    pub api_key: Option<String>,
}

impl GeminiConfig {
    pub fn build_client(&self, provider: ProviderId) -> Result<GeminiClient, ModelError> {
        Ok(
            GeminiClient::new(
                provider.clone(),
                gemini::Client::new(
                    match &self.api_key {
                        Some(key) => key.clone(),
                        None => env::var(API_KEY_ENV).map_err(|_| {
                            ModelError::configuration(format!(
                                "provider '{provider}' has no api_key and {API_KEY_ENV} is not set"
                            ))
                        })?,
                    }
                    .as_str(),
                )
                .map_err(|e| ModelError::configuration(e.to_string()))?,
            )
        )
    }
}
