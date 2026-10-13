// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::{
    config::fields::{
        spec::{FieldsSpec, IntoFieldSpecs},
        types::{AsFieldValue, FieldValue},
    },
    context::{ResolvedContextTool, ResolvedContextToolKind, ResolvedContextToolMcpTransport},
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

impl AsFieldValue for ResolvedContextToolMcpTransport {
    fn as_field_value(&self) -> FieldValue {
        FieldValue::Map {
            entries: match self {
                Self::Stdio { command, args, env } => BTreeMap::from([
                    ("type".to_string(), FieldValue::constant("stdio")),
                    ("command".to_string(), command.as_field_value()),
                    ("args".to_string(), args.as_field_value()),
                    ("env".to_string(), env.as_field_value()),
                ]),
                Self::Http { url, auth_token, headers } => BTreeMap::from([
                    ("type".to_string(), FieldValue::constant("http")),
                    ("url".to_string(), url.as_field_value()),
                    ("auth_token".to_string(), auth_token.as_field_value()),
                    ("headers".to_string(), headers.as_field_value()),
                ]),
            },
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
    use quote::quote;
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

    #[test]
    fn a_fixed_mcp_transport_is_one_constant() {
        let tokens = ResolvedContextToolMcpTransport::Http {
            url: RuntimeValue::constant("u".to_string()),
            auth_token: None,
            headers: HashMap::new(),
        }
        .as_field_value()
        .tokens(&["s".into()]);

        assert_eq!(
            tokens.loader().iter().map(ToString::to_string).collect::<Vec<_>>(),
            vec![quote! {
                .constant(
                    path!["s"],
                    serde_json::json!({ "auth_token": null, "headers": {}, "type": "http", "url": "u" })
                )
            }
            .to_string()],
        );
        assert!(tokens.mapper().is_empty());
    }

    #[test]
    fn a_runtime_mcp_value_seals_the_transport_and_aliases_the_value() {
        let tokens = ResolvedContextToolMcpTransport::Stdio {
            command: RuntimeValue::required_runtime("CMD"),
            args: vec![],
            env: HashMap::new(),
        }
        .as_field_value()
        .tokens(&["s".into()]);

        assert_eq!(
            tokens
                .loader()
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            vec![
                quote! { .seal(path!["s"]) }.to_string(),
                quote! { .constant(path!["s", "args"], serde_json::json!([])) }.to_string(),
                quote! { .constant(path!["s", "env"], serde_json::json!({})) }.to_string(),
                quote! { .constant(path!["s", "type"], serde_json::json!("stdio")) }.to_string(),
            ],
        );
        assert_eq!(
            tokens
                .mapper()
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            vec![quote! { .alias(path!["s", "command"], "CMD") }.to_string()],
        );
    }
}
