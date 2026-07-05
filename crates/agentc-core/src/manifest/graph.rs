// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};
use serde_json::{Value, to_value};

use agentc_blocks::graph::ReActGraphConfig;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ManifestAgentGraph {
    React(ReActGraphConfig),
}

impl ManifestAgentGraph {
    pub fn graph(&self) -> &str {
        match self {
            ManifestAgentGraph::React(_) => "react",
        }
    }

    pub fn config(&self) -> Value {
        match self {
            ManifestAgentGraph::React(config) => {
                to_value(config).expect("react graph config must serialize to JSON")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{manifest::Manifest, parser::format::SpecFormat};

    fn manifest_with_graph(graph: &str) -> String {
        format!(
            r#"
build {{
  archetype = "standalone"
}}

providers {{
  anthropic {{
    models = ["claude-haiku-4-5"]

    config {{
      api_key = "test"
    }}
  }}
}}

agent "assistant" {{
  version     = "0.1.0"
  description = "A helpful assistant."
  prompt      = "You are a helpful assistant."

  graph {{
{graph}
  }}

  model {{
    provider = "anthropic"
    name     = "claude-haiku-4-5"
  }}
}}
"#
        )
    }

    #[test]
    fn react_graph_name_and_config_are_serializable() {
        let graph = ManifestAgentGraph::React(ReActGraphConfig::default());

        assert_eq!(graph.graph(), "react");
        assert_eq!(graph.config(), Value::Object(Default::default()));
    }

    #[test]
    fn manifest_parses_react_graph() {
        let manifest = SpecFormat::hcl()
            .deserialize_string::<Manifest>(&manifest_with_graph(r#"    type = "react""#))
            .unwrap();

        let graph = &manifest
            .agent
            .get("assistant")
            .unwrap()
            .graph;

        assert_eq!(graph, &ManifestAgentGraph::React(ReActGraphConfig::default()));
    }

    #[test]
    fn manifest_rejects_unknown_graph() {
        let error = SpecFormat::hcl()
            .deserialize_string::<Manifest>(&manifest_with_graph(r#"    type = "unknown""#))
            .unwrap_err();

        assert!(
            error.to_string().contains("unknown"),
            "unexpected parser error: {error}"
        );
    }

    #[test]
    fn manifest_rejects_missing_graph() {
        let error = SpecFormat::hcl()
            .deserialize_string::<Manifest>(&manifest_with_graph(""))
            .unwrap_err();

        assert!(
            error.to_string().contains("graph"),
            "unexpected parser error: {error}"
        );
    }
}
