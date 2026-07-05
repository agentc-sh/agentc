// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use rig_core::providers::openrouter;
use serde::{Deserialize, Serialize};
use std::env;

use crate::{errors::ModelError, providers::openrouter::client::OpenRouterClient};

/// Configuration for constructing an [`OpenRouterClient`].
///
/// If `api_key` is `None`, reads from the `OPENROUTER_API_KEY` environment variable.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct OpenRouterConfig {
    pub api_key: Option<String>,
}

impl OpenRouterConfig {
    pub fn build_client(&self) -> Result<OpenRouterClient, ModelError> {
        Ok(OpenRouterClient::new(
            openrouter::Client::new(
                match &self.api_key {
                    Some(key) => key.clone(),
                    None => env::var("OPENROUTER_API_KEY")
                        .expect("OPENROUTER_API_KEY must be set if api_key is not provided"),
                }
                .as_str(),
            )
            .map_err(|e| ModelError::configuration(e.to_string()))?,
        ))
    }
}
