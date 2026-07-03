// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};
use serde_json::{Value, to_value};

use agentc_blocks::archetype::standalone::StandaloneArchetypeConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "archetype", rename_all = "snake_case")]
pub enum ManifestBuild {
    Standalone(StandaloneArchetypeConfig),
}

impl ManifestBuild {
    pub fn archetype(&self) -> &str {
        match self {
            ManifestBuild::Standalone(_) => "standalone",
        }
    }

    pub fn config(&self) -> Value {
        match self {
            ManifestBuild::Standalone(config) => to_value(config).unwrap_or(Value::Null),
        }
    }
}

impl Default for ManifestBuild {
    fn default() -> Self {
        Self::Standalone(StandaloneArchetypeConfig::default())
    }
}
