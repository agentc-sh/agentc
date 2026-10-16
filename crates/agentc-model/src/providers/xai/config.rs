// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use rig_core::{http_client::ReqwestClient, providers::xai};
use serde::{Deserialize, Serialize};
use std::env;

use crate::{
    errors::ModelError,
    providers::{
        rig::headers::{HeaderSource, Headers},
        xai::{client::XaiClient, constants::API_KEY_ENV},
    },
    types::identity::ProviderId,
};

/// Configuration for constructing an [`XaiClient`].
///
/// If `api_key` is `None`, reads from the `XAI_API_KEY` environment variable.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct XaiConfig {
    pub api_key: Option<String>,
    #[serde(default)]
    pub headers: Headers,
}

impl XaiConfig {
    pub fn with_header(mut self, name: impl Into<String>, source: HeaderSource) -> Self {
        self.headers = self.headers.with(name, source);
        self
    }

    pub fn build_client(&self, provider: ProviderId) -> Result<XaiClient, ModelError> {
        Ok(XaiClient::new(
            provider.clone(),
            xai::Client::builder().http_client(ReqwestClient::default()),
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
