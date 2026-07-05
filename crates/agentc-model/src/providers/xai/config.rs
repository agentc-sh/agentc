// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use rig_core::providers::xai;
use serde::{Deserialize, Serialize};
use std::env;

use crate::{errors::ModelError, providers::xai::client::XaiClient};

/// Configuration for constructing an [`XaiClient`].
///
/// If `api_key` is `None`, reads from the `XAI_API_KEY` environment variable.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct XaiConfig {
    pub api_key: Option<String>,
}

impl XaiConfig {
    pub fn build_client(&self) -> Result<XaiClient, ModelError> {
        Ok(XaiClient::new(
            xai::Client::new(
                match &self.api_key {
                    Some(key) => key.clone(),
                    None => env::var("XAI_API_KEY")
                        .expect("XAI_API_KEY must be set if api_key is not provided"),
                }
                .as_str(),
            )
            .map_err(|e| ModelError::configuration(e.to_string()))?,
        ))
    }
}
