// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use proc_macro2::TokenStream;
use quote::quote;

use agentc_compiler::generator::errors::GeneratorError;

use crate::{
    archetype::standalone::{codegen::agent::tools::ToolCodeGen, fields::FieldsSpec},
    context::{
        ResolvedContext, ResolvedContextToolBashEnv, ResolvedContextToolBashFsKind,
        ResolvedContextToolKind,
    },
};

/// All bash sandbox tools in the context. Each bash tool is registered independently
/// with its own command, filesystem, environment, resource, and network policies.
pub struct BashTools<'a>(pub &'a ResolvedContext);

impl ToolCodeGen for BashTools<'_> {
    fn imports(&self) -> Option<TokenStream> {
        self.0
            .tools
            .values()
            .any(|t| t.kind.is_bash())
            .then(|| {
                quote! {
                    use agentc_tools::bash::{BashTool, config::{CommandPolicy, EnvPolicy, FsPolicy, ExecLimits, NetworkPolicy}};
                }
            })
    }

    fn feature(&self) -> Option<&'static str> {
        self.0
            .tools
            .values()
            .any(|t| t.kind.is_bash())
            .then_some("bash")
    }

    /// Emits one `.with_typed_tool(BashTool::builder()...)` registration per Bash tool.
    fn registrations(&self, _fields: &FieldsSpec) -> Result<Vec<TokenStream>, GeneratorError> {
        let mut registrations = Vec::new();

        for tool in self.0.tools.values() {
            let ResolvedContextToolKind::Bash(bash) = &tool.kind else {
                continue;
            };

            let commands = &bash.commands;

            let command_policy = if commands.is_empty() {
                quote! { CommandPolicy::Unrestricted }
            } else {
                quote! { CommandPolicy::Allow(vec![#(#commands.to_string()),*]) }
            };

            let fs_policy = match &bash.fs.kind {
                ResolvedContextToolBashFsKind::InMemory => quote! { FsPolicy::InMemory },
                ResolvedContextToolBashFsKind::Overlay(path) => quote! {
                    FsPolicy::Overlay(::std::path::PathBuf::from(#path))
                },
                ResolvedContextToolBashFsKind::ReadWrite(path) => quote! {
                    FsPolicy::ReadWrite(::std::path::PathBuf::from(#path))
                },
            };

            let env_policy = match &bash.env {
                ResolvedContextToolBashEnv::Empty => quote! { EnvPolicy::Empty },
                ResolvedContextToolBashEnv::Inherit => quote! { EnvPolicy::Inherit },
                ResolvedContextToolBashEnv::Allow(vars) => quote! {
                    EnvPolicy::Allow(vec![#(#vars.to_string()),*])
                },
                ResolvedContextToolBashEnv::Deny(vars) => quote! {
                    EnvPolicy::Deny(vec![#(#vars.to_string()),*])
                },
            };

            let max_execution_time_secs = bash.limits.max_execution_time_secs;
            let max_output_size = bash.limits.max_output_size;
            let max_command_count = bash.limits.max_command_count;
            let max_loop_iterations = bash.limits.max_loop_iterations;

            let network_enabled = bash.network.enabled;
            let allowed_url_prefixes = &bash.network.allowed_url_prefixes;
            let allowed_methods = &bash.network.allowed_methods;
            let max_redirects = bash.network.max_redirects;
            let max_response_size = bash.network.max_response_size;
            let network_timeout_secs = bash.network.network_timeout_secs;

            let cwd = &bash.fs.cwd;

            registrations.push(quote! {
                builder = builder.with_typed_tool(
                    BashTool::builder()
                        .command_policy(#command_policy)
                        .fs_policy(#fs_policy)
                        .env_policy(#env_policy)
                        .cwd(#cwd)
                        .limits(ExecLimits {
                            max_execution_time: ::std::time::Duration::from_secs(#max_execution_time_secs),
                            max_output_size: #max_output_size,
                            max_command_count: #max_command_count,
                            max_loop_iterations: #max_loop_iterations,
                        })
                        .network(NetworkPolicy {
                            enabled: #network_enabled,
                            allowed_url_prefixes: vec![#(#allowed_url_prefixes.to_string()),*],
                            allowed_methods: ::std::collections::HashSet::from([#(#allowed_methods.to_string()),*]),
                            max_redirects: #max_redirects,
                            max_response_size: #max_response_size,
                            timeout: ::std::time::Duration::from_secs(#network_timeout_secs),
                        })
                        .build()
                );
            });
        }

        Ok(registrations)
    }
}
