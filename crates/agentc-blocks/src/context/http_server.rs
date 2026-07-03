// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};

use crate::types::RuntimeValue;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedContextHttpServer {
    /// The host address to bind the HTTP server to.
    pub host: RuntimeValue<String>,
    /// The port to bind the HTTP server to.
    pub port: RuntimeValue<u16>,
    /// Resolved protocols to include in the HTTP server.
    pub protocols: Vec<ResolvedContextHttpServerProtocol>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "config", rename_all = "snake_case")]
pub enum ResolvedContextHttpServerProtocol {
    AgUi(ResolvedContextHttpServerProtocolAgUi),
}

impl ResolvedContextHttpServerProtocol {
    pub fn as_ag_ui(&self) -> Option<&ResolvedContextHttpServerProtocolAgUi> {
        match self {
            ResolvedContextHttpServerProtocol::AgUi(protocol) => Some(protocol),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedContextHttpServerProtocolAgUi {
    /// The path to serve the AgUi interface on.
    pub path: String,
}

impl Default for ResolvedContextHttpServerProtocolAgUi {
    fn default() -> Self {
        Self { path: "/ag-ui".to_string() }
    }
}
