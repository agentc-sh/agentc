// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use rig_core::providers::gemini;
use serde::{Deserialize, Serialize};
use std::env;

use crate::{errors::ModelError, providers::gemini::client::GeminiClient};

/// Configuration for constructing a [`GeminiClient`].
///
/// If `api_key` is `None`, reads from the `GEMINI_API_KEY` environment variable.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct GeminiConfig {
    pub api_key: Option<String>,
}

impl GeminiConfig {
    pub fn build_client(&self) -> Result<GeminiClient, ModelError> {
        Ok(GeminiClient::new(
            gemini::Client::new(
                match &self.api_key {
                    Some(key) => key.clone(),
                    None => env::var("GEMINI_API_KEY")
                        .expect("GEMINI_API_KEY must be set if api_key is not provided"),
                }
                .as_str(),
            )
            .map_err(|e| ModelError::configuration(e.to_string()))?,
        ))
    }
}
