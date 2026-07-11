// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use proc_macro2::TokenStream;
use quote::quote;

use crate::{
    context::{
        ResolvedContext,
        ResolvedContextToolA2aTenant,
        ResolvedContextToolKind,
    },
    types::RuntimeValue,
};

pub struct A2aCodeGen;

impl A2aCodeGen {
    pub fn loader_calls(ctx: &ResolvedContext) -> TokenStream {
        let mut calls = Vec::<TokenStream>::new();

        for (name, tool) in &ctx.tools {
            let ResolvedContextToolKind::A2a(a2a) = &tool.kind else {
                continue;
            };

            Self::push_rv_loader(&["a2a", "agents", name, "url"], &a2a.url, &mut calls);
            Self::push_rv_loader_option(
                &["a2a", "agents", name, "auth_token"],
                a2a.auth_token.as_ref(),
                &mut calls,
            );

            for (key, value) in &a2a.headers {
                Self::push_rv_loader(
                    &["a2a", "agents", name, "headers", key],
                    value,
                    &mut calls,
                );
            }

            Self::push_tenant_loader(name, &a2a.tenant, &mut calls);
            Self::push_rv_loader_option(
                &["a2a", "agents", name, "timeout_secs"],
                a2a.timeout_secs.as_ref(),
                &mut calls,
            );

            for (index, mode) in a2a
                .default_accepted_output_modes
                .iter()
                .enumerate()
            {
                calls.push(quote! {
                    .constant(
                        path!["a2a", "agents", #name, "default_accepted_output_modes", #index],
                        serde_json::json!(#mode)
                    )
                });
            }

            if let Some(description) = &tool.description {
                calls.push(quote! {
                    .constant(
                        path!["a2a", "agents", #name, "description"],
                        serde_json::json!(#description)
                    )
                });
            }

            let capabilities = serde_json::to_string(&tool.capabilities)
                .unwrap()
                .parse::<TokenStream>()
                .unwrap();

            calls.push(quote! {
                .constant(
                    path!["a2a", "agents", #name, "capabilities"],
                    serde_json::json!(#capabilities)
                )
            });

            Self::push_bool_loader(
                &["a2a", "agents", name, "enabled"],
                &tool.enabled,
                &mut calls,
            );
        }

        quote! { #(#calls)* }
    }

    pub fn mapper_fields(ctx: &ResolvedContext) -> TokenStream {
        let mut fields = Vec::<TokenStream>::new();

        for (name, tool) in &ctx.tools {
            let ResolvedContextToolKind::A2a(a2a) = &tool.kind else {
                continue;
            };

            Self::push_rv_mapper(&["a2a", "agents", name, "url"], &a2a.url, &mut fields);
            Self::push_rv_mapper_option(
                &["a2a", "agents", name, "auth_token"],
                a2a.auth_token.as_ref(),
                &mut fields,
            );

            for (key, value) in &a2a.headers {
                Self::push_rv_mapper(
                    &["a2a", "agents", name, "headers", key],
                    value,
                    &mut fields,
                );
            }

            Self::push_tenant_mapper(name, &a2a.tenant, &mut fields);
            Self::push_rv_mapper_option(
                &["a2a", "agents", name, "timeout_secs"],
                a2a.timeout_secs.as_ref(),
                &mut fields,
            );
            Self::push_bool_mapper(
                &["a2a", "agents", name, "enabled"],
                &tool.enabled,
                &mut fields,
            );
        }

        quote! { #(#fields)* }
    }

    fn push_tenant_loader(
        name: &str,
        tenant: &ResolvedContextToolA2aTenant,
        calls: &mut Vec<TokenStream>,
    ) {
        match tenant {
            ResolvedContextToolA2aTenant::Inherit => {
                calls.push(quote! {
                    .constant(
                        path!["a2a", "agents", #name, "tenant", "policy"],
                        serde_json::json!("inherit")
                    )
                });
            }
            ResolvedContextToolA2aTenant::None => {
                calls.push(quote! {
                    .constant(
                        path!["a2a", "agents", #name, "tenant", "policy"],
                        serde_json::json!("none")
                    )
                });
            }
            ResolvedContextToolA2aTenant::Fixed { id } => {
                calls.push(quote! {
                    .constant(
                        path!["a2a", "agents", #name, "tenant", "policy"],
                        serde_json::json!("fixed")
                    )
                });

                Self::push_rv_loader(
                    &["a2a", "agents", name, "tenant", "id"],
                    id,
                    calls,
                );
            }
        }
    }

    fn push_tenant_mapper(
        name: &str,
        tenant: &ResolvedContextToolA2aTenant,
        fields: &mut Vec<TokenStream>,
    ) {
        if let ResolvedContextToolA2aTenant::Fixed { id } = tenant {
            Self::push_rv_mapper(
                &["a2a", "agents", name, "tenant", "id"],
                id,
                fields,
            );
        }
    }

    fn push_rv_loader_option(
        path: &[&str],
        rv: Option<&RuntimeValue<String>>,
        calls: &mut Vec<TokenStream>,
    ) {
        if let Some(rv) = rv {
            Self::push_rv_loader(path, rv, calls);
        }
    }

    fn push_rv_loader(
        path: &[&str],
        rv: &RuntimeValue<String>,
        calls: &mut Vec<TokenStream>,
    ) {
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

    fn push_bool_loader(
        path: &[&str],
        rv: &RuntimeValue<bool>,
        calls: &mut Vec<TokenStream>,
    ) {
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

    fn push_rv_mapper_option(
        path: &[&str],
        rv: Option<&RuntimeValue<String>>,
        fields: &mut Vec<TokenStream>,
    ) {
        if let Some(rv) = rv {
            Self::push_rv_mapper(path, rv, fields);
        }
    }

    fn push_rv_mapper(
        path: &[&str],
        rv: &RuntimeValue<String>,
        fields: &mut Vec<TokenStream>,
    ) {
        let path_segments = path.to_vec();

        if let RuntimeValue::Runtime { env, .. } = rv {
            fields.push(quote! {
                .field(path![#(#path_segments),*], #env)
            });
        }
    }

    fn push_bool_mapper(
        path: &[&str],
        rv: &RuntimeValue<bool>,
        fields: &mut Vec<TokenStream>,
    ) {
        let path_segments = path.to_vec();

        if let RuntimeValue::Runtime { env, .. } = rv {
            fields.push(quote! {
                .field(path![#(#path_segments),*], #env)
            });
        }
    }
}
