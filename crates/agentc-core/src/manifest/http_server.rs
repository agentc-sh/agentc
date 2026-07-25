// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use sanitizer::prelude::*;
use serde::{Deserialize, Serialize};
use validator::Validate;

use agentc_blocks::types::RuntimeValue;

#[derive(Debug, Clone, Serialize, Deserialize, Validate, Sanitizer)]
#[serde(default)]
pub struct ManifestHttpServer {
    /// The host address to bind the server to.
    pub host: RuntimeValue<String>,
    /// The port to bind the server to.
    pub port: RuntimeValue<u16>,
    /// The maximum size, in bytes, of an accepted request body.
    pub max_request_size: RuntimeValue<usize>,
    /// Additional protocols to include in the HTTP server.
    #[serde(default)]
    #[validate(nested)]
    pub protocol: Option<ManifestHttpServerProtocol>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, Sanitizer)]
pub struct ManifestHttpServerProtocol {
    #[serde(default)]
    #[validate(nested)]
    pub ag_ui: Option<ManifestHttpServerProtocolAgUi>,
    #[serde(default)]
    #[validate(nested)]
    pub a2a: Option<ManifestHttpServerProtocolA2a>,
}

impl Default for ManifestHttpServer {
    fn default() -> Self {
        Self {
            host: RuntimeValue::default_runtime("HTTP_HOST", "127.0.0.1".to_string()),
            port: RuntimeValue::default_runtime("HTTP_PORT", 8080u16),
            max_request_size: RuntimeValue::default_runtime("HTTP_MAX_REQUEST_SIZE", 2097152usize),
            protocol: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, Sanitizer)]
pub struct ManifestHttpServerProtocolAgUi {
    /// The path to serve the AG UI on.
    #[serde(default = "default_ag_ui_path")]
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, Sanitizer)]
pub struct ManifestHttpServerProtocolA2a {
    /// The path to serve A2A on.
    #[serde(default = "default_a2a_path")]
    pub path: String,
}

fn default_ag_ui_path() -> String {
    "/ag-ui".to_string()
}

fn default_a2a_path() -> String {
    "/a2a".to_string()
}
