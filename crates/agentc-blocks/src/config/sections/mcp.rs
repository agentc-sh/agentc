// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use proc_macro2::TokenStream;
use quote::quote;

use agentc_compiler::generator::{
    blocks::fragment::{Fragment, FragmentBlock},
    context::GenerationContext,
    errors::GeneratorError,
    extension::ErasedContributionValue,
};

use crate::{
    config::sections::{
        block::ConfigSectionBlockBuilderExt,
        contribution::{ConfigSectionContribution, ConfigSections},
    },
    context::{ResolvedContext, ResolvedContextToolKind, ResolvedContextToolMcpTransport},
    types::RuntimeValue,
};

pub struct McpSection;

impl McpSection {
    pub const NAME: &'static str = "mcp";

    pub fn block(id: &'static str) -> FragmentBlock<ResolvedContext> {
        FragmentBlock::<ResolvedContext>::builder()
            .id(id)
            .contribute_config_sections()
            .build(Self)
    }

    fn section(&self, ctx: &ResolvedContext) -> Result<ConfigSections, GeneratorError> {
        ConfigSections::from_entries([
            ConfigSectionContribution::new(Self::NAME)
                .types(quote! {
                    #[derive(Debug, Clone, Serialize, Deserialize)]
                    #[serde(tag = "type", rename_all = "snake_case")]
                    pub enum McpTransportConfig {
                        Stdio {
                            command: String,
                            #[serde(default)]
                            args: Vec<String>,
                            #[serde(default)]
                            env: HashMap<String, String>,
                        },
                        Http {
                            url: String,
                            #[serde(default)]
                            auth_token: Option<String>,
                            #[serde(default)]
                            headers: HashMap<String, String>,
                        },
                    }

                    #[derive(Debug, Clone, Serialize, Deserialize, Default)]
                    #[serde(default)]
                    pub struct McpConfig {
                        pub servers: HashMap<String, McpTransportConfig>,
                    }
                })
                .fields(quote! {
                    pub mcp: McpConfig,
                })
                .loader(Self::loader_calls(ctx))
                .mapper(Self::mapper_fields(ctx)),
        ])
        .map_err(|error| GeneratorError::unexpected(error.to_string()))
    }

    fn loader_calls(ctx: &ResolvedContext) -> TokenStream {
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

    fn mapper_fields(ctx: &ResolvedContext) -> TokenStream {
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

impl Fragment<ResolvedContext> for McpSection {
    fn generate_contribution(
        &self,
        ctx: &GenerationContext<ResolvedContext>,
        point: &str,
    ) -> Result<ErasedContributionValue, GeneratorError> {
        match point {
            "config::sections::use"
            | "config::sections::types"
            | "config::sections::fields"
            | "config::sections::loader"
            | "config::sections::mapper" => {
                Ok(ErasedContributionValue::new(self.section(ctx.as_inner())?))
            }
            _ => Err(GeneratorError::unexpected(format!("Unknown extension point '{}'", point))),
        }
    }
}
