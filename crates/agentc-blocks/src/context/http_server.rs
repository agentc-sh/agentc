// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};
use serde_json::{Value, to_value};

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
    A2a(ResolvedContextHttpServerProtocolA2a),
}

impl ResolvedContextHttpServerProtocol {
    pub fn as_ag_ui(&self) -> Option<&ResolvedContextHttpServerProtocolAgUi> {
        match self {
            ResolvedContextHttpServerProtocol::AgUi(protocol) => Some(protocol),
            _ => None,
        }
    }

    pub fn as_a2a(&self) -> Option<&ResolvedContextHttpServerProtocolA2a> {
        match self {
            ResolvedContextHttpServerProtocol::A2a(protocol) => Some(protocol),
            _ => None,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            ResolvedContextHttpServerProtocol::AgUi(_) => "ag_ui",
            ResolvedContextHttpServerProtocol::A2a(_) => "a2a",
        }
    }

    pub fn config(&self) -> Value {
        match self {
            ResolvedContextHttpServerProtocol::AgUi(config) => {
                to_value(config).expect("ag_ui protocol config must serialize to JSON")
            }
            ResolvedContextHttpServerProtocol::A2a(config) => {
                to_value(config).expect("a2a protocol config must serialize to JSON")
            }
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedContextHttpServerProtocolA2a {
    /// The path to serve the A2A interface on.
    pub path: String,
}

impl Default for ResolvedContextHttpServerProtocolA2a {
    fn default() -> Self {
        Self { path: "/a2a".to_string() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_and_config_identify_ag_ui() {
        let protocol =
            ResolvedContextHttpServerProtocol::AgUi(ResolvedContextHttpServerProtocolAgUi {
                path: "/custom".to_string(),
            });

        assert_eq!(protocol.name(), "ag_ui");
        assert_eq!(protocol.config(), serde_json::json!({ "path": "/custom" }));
    }

    #[test]
    fn name_and_config_identify_a2a() {
        let protocol =
            ResolvedContextHttpServerProtocol::A2a(ResolvedContextHttpServerProtocolA2a {
                path: "/custom".to_string(),
            });

        assert_eq!(protocol.name(), "a2a");
        assert_eq!(protocol.config(), serde_json::json!({ "path": "/custom" }));
    }
}
