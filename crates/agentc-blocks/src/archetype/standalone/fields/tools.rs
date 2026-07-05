// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use crate::{
    archetype::standalone::fields::spec::{FieldsSpec, IntoFieldSpecs},
    context::{ResolvedContextTool, ResolvedContextToolKind},
};

/// Pairs a tool name with its resolved context for use with [`IntoFieldSpecs`].
///
/// Tool field paths include the tool name (e.g. `["tool", "my_tool", "enabled"]`),
/// so the name must accompany the tool when building field specs.
pub struct NamedTool<'a>(pub &'a str, pub &'a ResolvedContextTool);

impl IntoFieldSpecs for NamedTool<'_> {
    fn extend_fields(&self, fields: &mut FieldsSpec) {
        let (name, tool) = (self.0, self.1);

        match &tool.kind {
            ResolvedContextToolKind::Javascript(_) | ResolvedContextToolKind::Python(_) => {
                fields.push(&["tool", name, "enabled"], &tool.enabled);

                for (config_key, config_value) in &tool.config {
                    fields.push(&["tool", name, config_key.as_str()], config_value);
                }
            }

            // MCP loader calls are contributed to `config::loader` by AgentCodeGen.
            ResolvedContextToolKind::Mcp(_) => {}

            // Bash tools have no runtime-configurable fields beyond what is baked at compile time.
            ResolvedContextToolKind::Bash(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        context::{
            ResolvedContextToolJavascript, ResolvedContextToolMcp, ResolvedContextToolMcpTransport,
        },
        types::RuntimeValue,
    };
    use std::collections::HashMap;

    fn tool(
        kind: ResolvedContextToolKind,
        config: HashMap<String, RuntimeValue<String>>,
    ) -> ResolvedContextTool {
        ResolvedContextTool {
            name: "t".to_string(),
            description: None,
            enabled: RuntimeValue::constant(true),
            capabilities: vec![],
            config,
            kind,
        }
    }

    #[test]
    fn javascript_tool_registers_enabled_and_each_config_key() {
        let mut config = HashMap::new();
        config.insert("api_url".to_string(), RuntimeValue::constant("u".to_string()));

        let js = tool(
            ResolvedContextToolKind::Javascript(ResolvedContextToolJavascript {
                bundle_path: "bundle.js".to_string(),
                export_name: "run".to_string(),
            }),
            config,
        );

        let fields = FieldsSpec::collect_from(&NamedTool("mytool", &js));

        assert!(fields.get(&["tool", "mytool", "enabled"]).is_some());
        assert!(fields.get(&["tool", "mytool", "api_url"]).is_some());
        assert_eq!(fields.as_inner().len(), 2);
    }

    #[test]
    fn mcp_tool_registers_no_fields() {
        let mcp = tool(
            ResolvedContextToolKind::Mcp(ResolvedContextToolMcp {
                transport: ResolvedContextToolMcpTransport::Stdio {
                    command: RuntimeValue::constant("cmd".to_string()),
                    args: vec![],
                    env: HashMap::new(),
                },
            }),
            HashMap::new(),
        );

        let fields = FieldsSpec::collect_from(&NamedTool("server", &mcp));

        assert!(fields.as_inner().is_empty());
    }
}
