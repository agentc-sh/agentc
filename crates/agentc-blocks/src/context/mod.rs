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
