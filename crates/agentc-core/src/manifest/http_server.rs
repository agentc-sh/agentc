// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use sanitizer::prelude::*;
use serde::{Deserialize, Serialize};
use validator::Validate;

use agentc_blocks::types::RuntimeValue;

#[derive(Debug, Clone, Serialize, Deserialize, Validate, Sanitizer)]
pub struct ManifestHttpServer {
    /// The host address to bind the server to.
    #[serde(default = "default_host")]
    pub host: RuntimeValue<String>,
    /// The port to bind the server to.
    #[serde(default = "default_port")]
    pub port: RuntimeValue<u16>,
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
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, Sanitizer)]
pub struct ManifestHttpServerProtocolAgUi {
    /// The path to serve the AG UI on.
    #[serde(default = "default_ag_ui_path")]
    pub path: String,
}

fn default_host() -> RuntimeValue<String> {
    RuntimeValue::default_runtime("HTTP_HOST", "127.0.0.1".to_string())
}

fn default_port() -> RuntimeValue<u16> {
    RuntimeValue::default_runtime("HTTP_PORT", 8080u16)
}

fn default_ag_ui_path() -> String {
    "/ag-ui".to_string()
}

impl Default for ManifestHttpServer {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            protocol: None,
        }
    }
}
