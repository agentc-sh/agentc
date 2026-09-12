// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use proc_macro2::TokenStream;
use quote::quote;

use agentc_compiler::generator::errors::GeneratorError;

use crate::{
    config::fields::FieldsSpec,
    context::{ResolvedContext, ResolvedContextToolBashEnv, ResolvedContextToolKind},
    contributions::import::ImportContribution,
    graph::codegen::tools::ToolCodeGen,
};

/// All Bash tools in the context.
pub struct BashTools<'a>(pub &'a ResolvedContext);

impl ToolCodeGen for BashTools<'_> {
    fn imports(&self) -> Vec<ImportContribution> {
        self.0
            .tools
            .values()
            .any(|tool| tool.kind.is_bash())
            .then(|| {
                vec![
                    ImportContribution::path(&["agentc_tools", "bash"]).item("BashTool"),
                    ImportContribution::path(&["agentc_tools", "bash", "config"])
                        .item("CommandPolicy")
                        .item("EnvPolicy")
                        .item("ExecLimits"),
                ]
            })
            .unwrap_or_default()
    }

    fn feature(&self) -> Option<&'static str> {
        self.0
            .tools
            .values()
            .any(|tool| tool.kind.is_bash())
            .then_some("bash")
    }

    fn registrations(&self, _fields: &FieldsSpec) -> Result<Vec<TokenStream>, GeneratorError> {
        let mut registrations = Vec::new();

        for tool in self.0.tools.values() {
            let ResolvedContextToolKind::Bash(bash) = &tool.kind else {
                continue;
            };

            let command_policy = if bash.commands.is_empty() {
                quote! { CommandPolicy::Unrestricted }
            } else {
                let commands = &bash.commands;

                quote! { CommandPolicy::Allow(vec![#(#commands.to_string()),*]) }
            };
            let env_policy = match &bash.env {
                ResolvedContextToolBashEnv::Empty => quote! { EnvPolicy::Empty },
                ResolvedContextToolBashEnv::Inherit => quote! { EnvPolicy::Inherit },
                ResolvedContextToolBashEnv::Allow(vars) => {
                    quote! { EnvPolicy::Allow(vec![#(#vars.to_string()),*]) }
                }
                ResolvedContextToolBashEnv::Deny(vars) => {
                    quote! { EnvPolicy::Deny(vec![#(#vars.to_string()),*]) }
                }
            };

            let cwd = &bash.cwd;
            let max_execution_time_secs = bash.limits.max_execution_time_secs;
            let max_output_size = bash.limits.max_output_size;
            let max_command_count = bash.limits.max_command_count;
            let max_loop_iterations = bash.limits.max_loop_iterations;
            let shared = bash
                .shared
                .then(|| quote! { .shared() });

            registrations.push(quote! {
                builder = builder.with_typed_tool(
                    BashTool::builder(fs.clone(), http.clone())
                        .command_policy(#command_policy)
                        .cwd(#cwd)
                        .env_policy(#env_policy)
                        .limits(ExecLimits {
                            max_execution_time: ::std::time::Duration::from_secs(
                                #max_execution_time_secs
                            ),
                            max_output_size: #max_output_size,
                            max_command_count: #max_command_count,
                            max_loop_iterations: #max_loop_iterations,
                        })
                        #shared
                        .build()
                );
            });
        }

        Ok(registrations)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::{
        config::fields::FieldsSpec,
        context::{ResolvedContext, ResolvedContextToolKind},
        contributions::import::ImportContribution,
        graph::codegen::tools::{ToolCodeGen, bash::BashTools},
    };

    struct BashToolsFixture;

    impl BashToolsFixture {
        fn context() -> ResolvedContext {
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
                "tools": {
                    "shell": {
                        "name": "shell",
                        "description": null,
                        "enabled": true,
                        "capabilities": [],
                        "config": {},
                        "kind": {
                            "kind": "bash",
                            "commands": ["git", "rg"],
                            "cwd": "/workspace",
                            "env": { "allow": ["HOME", "PATH"] },
                            "limits": {
                                "max_execution_time_secs": 7,
                                "max_output_size": 512,
                                "max_command_count": 23,
                                "max_loop_iterations": 29
                            },
                            "shared": true
                        }
                    }
                },
                "skills": {},
                "http_server": null
            }))
            .unwrap()
        }

        fn registrations(context: &ResolvedContext) -> String {
            BashTools(context)
                .registrations(&FieldsSpec::collect_from(context))
                .expect("Bash registrations should be generated")
                .into_iter()
                .next()
                .expect("Bash registration should exist")
                .to_string()
        }
    }

    #[test]
    fn imports_bashkit_tool_contract() {
        assert_eq!(
            BashTools(&BashToolsFixture::context()).imports(),
            vec![
                ImportContribution::path(&["agentc_tools", "bash"]).item("BashTool"),
                ImportContribution::path(&["agentc_tools", "bash", "config"])
                    .item("CommandPolicy")
                    .item("EnvPolicy")
                    .item("ExecLimits"),
            ],
        );
    }

    #[test]
    fn registers_configured_bashkit_tool_with_process_resources() {
        let registration = BashToolsFixture::registrations(&BashToolsFixture::context());

        assert!(registration.contains("BashTool :: builder (fs . clone () , http . clone ())"));
        assert!(registration.contains("CommandPolicy :: Allow"));
        assert!(registration.contains("\"git\""));
        assert!(registration.contains("\"rg\""));
        assert!(registration.contains("EnvPolicy :: Allow"));
        assert!(registration.contains("\"HOME\""));
        assert!(registration.contains("\"PATH\""));
        assert!(registration.contains("cwd (\"/workspace\")"));
        assert!(registration.contains("std :: time :: Duration :: from_secs (7"));
        assert!(registration.contains("max_output_size : 512"));
        assert!(registration.contains("max_command_count : 23"));
        assert!(registration.contains("max_loop_iterations : 29"));
        assert!(registration.contains("shared ()"));
        assert!(registration.contains("build ()"));
        assert!(!registration.contains("fs_policy"));
        assert!(!registration.contains("NetworkPolicy"));
        assert!(!registration.contains("network ("));
        assert!(!registration.contains("allowed_url_prefixes"));
        assert!(!registration.contains("allowed_methods"));
    }

    #[test]
    fn omits_shared_scope_for_factory_bash_tool() {
        let mut context = BashToolsFixture::context();
        let ResolvedContextToolKind::Bash(bash) = &mut context
            .tools
            .get_mut("shell")
            .expect("shell tool should exist")
            .kind
        else {
            panic!("shell tool should be Bash");
        };

        bash.shared = false;

        assert!(!BashToolsFixture::registrations(&context).contains("shared ()"));
    }

    #[test]
    fn emits_unrestricted_policy_for_empty_command_list() {
        let mut context = BashToolsFixture::context();
        let ResolvedContextToolKind::Bash(bash) = &mut context
            .tools
            .get_mut("shell")
            .expect("shell tool should exist")
            .kind
        else {
            panic!("shell tool should be Bash");
        };

        bash.commands.clear();

        assert!(
            BashToolsFixture::registrations(&context).contains("CommandPolicy :: Unrestricted")
        );
    }

    #[test]
    fn contributes_bash_feature_only_when_bash_tools_exist() {
        let mut context = BashToolsFixture::context();

        assert_eq!(BashTools(&context).feature(), Some("bash"));

        context.tools.clear();

        assert!(BashTools(&context).imports().is_empty());
        assert!(BashTools(&context).feature().is_none());
        assert!(
            BashTools(&context)
                .registrations(&FieldsSpec::collect_from(&context))
                .expect("empty Bash registrations should generate")
                .is_empty()
        );
    }
}
