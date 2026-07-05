// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use proc_macro2::TokenStream;
use quote::quote;

use crate::{
    context::{ResolvedContext, ResolvedContextToolKind, ResolvedContextToolMcpTransport},
    types::RuntimeValue,
};

/// Generates the config loader calls and mapper fields for MCP server tools.
pub struct McpCodeGen;

impl McpCodeGen {
    pub fn loader_calls(ctx: &ResolvedContext) -> TokenStream {
        let mut calls = Vec::<TokenStream>::new();

        for (name, tool) in &ctx.tools {
            let ResolvedContextToolKind::Mcp(mcp) = &tool.kind else {
                continue;
            };

            match &mcp.transport {
                ResolvedContextToolMcpTransport::Stdio { command, args, env } => {
                    calls.push(quote! {
                        .constant(
                            path!["mcp", "servers", #name, "type"],
                            serde_json::json!("stdio")
                        )
                    });

                    Self::push_rv_loader(&["mcp", "servers", name, "command"], command, &mut calls);

                    for (i, arg) in args.iter().enumerate() {
                        Self::push_rv_loader_indexed(
                            &["mcp", "servers", name, "args"],
                            i,
                            arg,
                            &mut calls,
                        );
                    }

                    for (key, value) in env {
                        Self::push_rv_loader(
                            &["mcp", "servers", name, "env", key],
                            value,
                            &mut calls,
                        );
                    }
                }

                ResolvedContextToolMcpTransport::Http { url, auth_token, headers } => {
                    calls.push(quote! {
                        .constant(
                            path!["mcp", "servers", #name, "type"],
                            serde_json::json!("http")
                        )
                    });

                    Self::push_rv_loader(&["mcp", "servers", name, "url"], url, &mut calls);

                    if let Some(token) = auth_token {
                        Self::push_rv_loader(
                            &["mcp", "servers", name, "auth_token"],
                            token,
                            &mut calls,
                        );
                    }

                    for (key, value) in headers {
                        Self::push_rv_loader(
                            &["mcp", "servers", name, "headers", key],
                            value,
                            &mut calls,
                        );
                    }
                }
            }
        }

        quote! { #(#calls)* }
    }

    pub fn mapper_fields(ctx: &ResolvedContext) -> TokenStream {
        let mut fields = Vec::<TokenStream>::new();

        for (name, tool) in &ctx.tools {
            let ResolvedContextToolKind::Mcp(mcp) = &tool.kind else {
                continue;
            };

            match &mcp.transport {
                ResolvedContextToolMcpTransport::Stdio { command, args, env } => {
                    Self::push_rv_mapper(
                        &["mcp", "servers", name, "command"],
                        command,
                        &mut fields,
                    );

                    for (i, arg) in args.iter().enumerate() {
                        Self::push_rv_mapper_indexed(
                            &["mcp", "servers", name, "args"],
                            i,
                            arg,
                            &mut fields,
                        );
                    }

                    for (key, value) in env {
                        Self::push_rv_mapper(
                            &["mcp", "servers", name, "env", key],
                            value,
                            &mut fields,
                        );
                    }
                }

                ResolvedContextToolMcpTransport::Http { url, auth_token, headers } => {
                    Self::push_rv_mapper(&["mcp", "servers", name, "url"], url, &mut fields);

                    if let Some(token) = auth_token {
                        Self::push_rv_mapper(
                            &["mcp", "servers", name, "auth_token"],
                            token,
                            &mut fields,
                        );
                    }

                    for (key, value) in headers {
                        Self::push_rv_mapper(
                            &["mcp", "servers", name, "headers", key],
                            value,
                            &mut fields,
                        );
                    }
                }
            }
        }

        quote! { #(#fields)* }
    }

    fn push_rv_loader(path: &[&str], rv: &RuntimeValue<String>, calls: &mut Vec<TokenStream>) {
        let path_segments = path.to_vec();

        match rv {
            RuntimeValue::Constant(value) => {
                calls.push(quote! {
                    .constant(
                        path![#(#path_segments),*],
                        serde_json::json!(#value)
                    )
                });
            }
            RuntimeValue::Runtime { default, .. } => {
                if let Some(default) = default {
                    calls.push(quote! {
                        .default(
                            path![#(#path_segments),*],
                            serde_json::json!(#default)
                        )
                    });
                }
            }
        }
    }

    fn push_rv_loader_indexed(
        base_path: &[&str],
        index: usize,
        rv: &RuntimeValue<String>,
        calls: &mut Vec<TokenStream>,
    ) {
        let base_segments = base_path.to_vec();

        match rv {
            RuntimeValue::Constant(value) => {
                calls.push(quote! {
                    .constant(
                        path![#(#base_segments),*, #index],
                        serde_json::json!(#value)
                    )
                });
            }
            RuntimeValue::Runtime { default, .. } => {
                if let Some(default) = default {
                    calls.push(quote! {
                        .default(
                            path![#(#base_segments),*, #index],
                            serde_json::json!(#default)
                        )
                    });
                }
            }
        }
    }

    fn push_rv_mapper(path: &[&str], rv: &RuntimeValue<String>, fields: &mut Vec<TokenStream>) {
        let path_segments = path.to_vec();

        if let RuntimeValue::Runtime { env, .. } = rv {
            fields.push(quote! {
                .field(path![#(#path_segments),*], #env)
            });
        }
    }

    fn push_rv_mapper_indexed(
        base_path: &[&str],
        index: usize,
        rv: &RuntimeValue<String>,
        fields: &mut Vec<TokenStream>,
    ) {
        let base_segments = base_path.to_vec();

        if let RuntimeValue::Runtime { env, .. } = rv {
            fields.push(quote! {
                .field(path![#(#base_segments),*, #index], #env)
            });
        }
    }
}
