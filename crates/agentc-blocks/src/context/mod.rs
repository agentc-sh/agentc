// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

pub mod agent;
pub mod block;
pub mod http_server;
pub mod observability;
pub mod provider;
pub mod runtime;
pub mod skill;
pub mod tool;

pub use agent::*;
pub use block::*;
pub use http_server::*;
pub use provider::*;
pub use runtime::*;
pub use skill::*;
pub use tool::*;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedContext {
    /// Normalized name (snake_case), derived from the agent label.
    pub slug: String,
    /// The original agent label as declared in the manifest.
    pub agent_name: String,
    /// The runtime configuration for information not specific to any components.
    pub runtime: ResolvedContextRuntime,
    /// The resolved providers configuration.
    pub providers: Vec<ResolvedContextProvider>,
    /// The resolved agent context.
    pub agent: ResolvedContextAgent,
    /// Resolved custom block contexts, keyed by block label.
    pub blocks: HashMap<String, ResolvedContextBlock>,
    /// Resolved tool contexts, keyed by tool name.
    pub tools: HashMap<String, ResolvedContextTool>,
    /// Resolved skill contexts, keyed by skill name.
    pub skills: HashMap<String, ResolvedContextSkill>,
    /// Optional HTTP server configuration.
    pub http_server: Option<ResolvedContextHttpServer>,
}

impl ResolvedContext {
    /// Whether any component in this context is implemented in TypeScript.
    pub fn has_typescript_components(&self) -> bool {
        self.tools
            .values()
            .any(|tool| tool.kind.is_javascript())
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn context(tools: serde_json::Value) -> ResolvedContext {
        serde_json::from_value(json!({
            "slug": "assistant",
            "agent_name": "assistant",
            "runtime": { "default_tenant_id": "default" },
            "providers": [],
            "agent": {
                "version": "0.1.0",
                "description": null,
                "prompt": null,
                "capabilities": null,
                "capability_policy": null,
                "model": { "provider": "anthropic", "name": "claude" }
            },
            "blocks": {},
            "tools": tools,
            "skills": {},
            "http_server": null
        }))
        .unwrap()
    }

    #[test]
    fn typescript_components_are_detected_from_javascript_tools() {
        assert!(
            context(json!({
                "search": {
                    "name": "search",
                    "description": null,
                    "enabled": true,
                    "capabilities": [],
                    "config": {},
                    "kind": {
                        "kind": "javascript",
                        "bundle_path": "/artifacts/search/dist/index.js",
                        "export_name": "search"
                    }
                }
            }))
            .has_typescript_components()
        );
    }

    #[test]
    fn no_typescript_components_without_a_javascript_tool() {
        assert!(!context(json!({})).has_typescript_components());
    }
}
