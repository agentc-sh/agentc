// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use crate::{
    config::fields::spec::{FieldsSpec, IntoFieldSpecs},
    context::{ResolvedContextTool, ResolvedContextToolKind},
};

impl IntoFieldSpecs for ResolvedContextTool {
    fn extend_fields(&self, fields: &mut FieldsSpec) {
        match &self.kind {
            ResolvedContextToolKind::Javascript(_)
            | ResolvedContextToolKind::Python(_)
            | ResolvedContextToolKind::Bash(_) => {
                let key = self.config_key();

                fields.push(&["tool", key.as_str(), "enabled"], &self.enabled);

                for (config_key, config_value) in &self.config {
                    fields.push(&["tool", key.as_str(), config_key.as_str()], config_value);
                }
            }

            // MCP loader calls are contributed to `config::loader` by AgentCodeGen.
            ResolvedContextToolKind::Mcp(_) => {}

            // NOTE: Determine what is needed here, if at all for A2A
            ResolvedContextToolKind::A2a(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        context::{
            ResolvedContextToolBash, ResolvedContextToolBashEnv, ResolvedContextToolBashLimits,
            ResolvedContextToolJavascript, ResolvedContextToolMcp, ResolvedContextToolMcpTransport,
        },
        types::RuntimeValue,
    };
    use std::collections::HashMap;

    struct ToolFieldsFixture;

    impl ToolFieldsFixture {
        fn tool(
            name: &str,
            kind: ResolvedContextToolKind,
            config: HashMap<String, RuntimeValue<String>>,
        ) -> ResolvedContextTool {
            ResolvedContextTool {
                name: name.to_string(),
                description: None,
                enabled: RuntimeValue::constant(true),
                capabilities: vec![],
                config,
                kind,
            }
        }
    }

    #[test]
    fn javascript_tool_registers_enabled_and_each_config_key() {
        let mut config = HashMap::new();
        config.insert("api_url".to_string(), RuntimeValue::constant("u".to_string()));

        let js = ToolFieldsFixture::tool(
            "mytool",
            ResolvedContextToolKind::Javascript(ResolvedContextToolJavascript {
                bundle_path: "bundle.js".to_string(),
                export_name: "run".to_string(),
            }),
            config,
        );

        let fields = FieldsSpec::collect_from(&js);

        assert!(
            fields
                .get(&["tool", "mytool", "enabled"])
                .is_some()
        );
        assert!(
            fields
                .get(&["tool", "mytool", "api_url"])
                .is_some()
        );
        assert_eq!(fields.as_inner().len(), 2);
    }

    #[test]
    fn bash_tool_registers_enabled_and_each_config_key() {
        let fields = FieldsSpec::collect_from(&ToolFieldsFixture::tool(
            "shell",
            ResolvedContextToolKind::Bash(ResolvedContextToolBash {
                commands: vec![],
                cwd: "/workspace".to_string(),
                env: ResolvedContextToolBashEnv::Empty,
                limits: ResolvedContextToolBashLimits {
                    max_execution_time_secs: 7,
                    max_output_size: 512,
                    max_command_count: 23,
                    max_loop_iterations: 29,
                },
                shared: false,
            }),
            HashMap::from([("api_url".to_string(), RuntimeValue::constant("u".to_string()))]),
        ));

        assert!(
            fields
                .get(&["tool", "shell", "enabled"])
                .is_some()
        );
        assert!(
            fields
                .get(&["tool", "shell", "api_url"])
                .is_some()
        );
        assert_eq!(fields.as_inner().len(), 2);
    }

    #[test]
    fn mcp_tool_registers_no_fields() {
        let mcp = ToolFieldsFixture::tool(
            "server",
            ResolvedContextToolKind::Mcp(ResolvedContextToolMcp {
                transport: ResolvedContextToolMcpTransport::Stdio {
                    command: RuntimeValue::constant("cmd".to_string()),
                    args: vec![],
                    env: HashMap::new(),
                },
            }),
            HashMap::new(),
        );

        let fields = FieldsSpec::collect_from(&mcp);

        assert!(fields.as_inner().is_empty());
    }
}
