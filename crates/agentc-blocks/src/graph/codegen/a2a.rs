// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use proc_macro2::TokenStream;
use quote::quote;

use crate::{
    context::{ResolvedContext, ResolvedContextToolA2aTenant, ResolvedContextToolKind},
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
                Self::push_rv_loader(&["a2a", "agents", name, "headers", key], value, &mut calls);
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

            Self::push_rv_loader(&["a2a", "agents", name, "enabled"], &tool.enabled, &mut calls);
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
                Self::push_rv_mapper(&["a2a", "agents", name, "headers", key], value, &mut fields);
            }

            Self::push_tenant_mapper(name, &a2a.tenant, &mut fields);
            Self::push_rv_mapper_option(
                &["a2a", "agents", name, "timeout_secs"],
                a2a.timeout_secs.as_ref(),
                &mut fields,
            );
            Self::push_rv_mapper(&["a2a", "agents", name, "enabled"], &tool.enabled, &mut fields);
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

                Self::push_rv_loader(&["a2a", "agents", name, "tenant", "id"], id, calls);
            }
        }
    }

    fn push_tenant_mapper(
        name: &str,
        tenant: &ResolvedContextToolA2aTenant,
        fields: &mut Vec<TokenStream>,
    ) {
        if let ResolvedContextToolA2aTenant::Fixed { id } = tenant {
            Self::push_rv_mapper(&["a2a", "agents", name, "tenant", "id"], id, fields);
        }
    }

    fn push_rv_loader_option<T>(
        path: &[&str],
        rv: Option<&RuntimeValue<T>>,
        calls: &mut Vec<TokenStream>,
    ) where
        T: serde::Serialize,
    {
        if let Some(rv) = rv {
            Self::push_rv_loader(path, rv, calls);
        }
    }

    fn push_rv_loader<T>(path: &[&str], rv: &RuntimeValue<T>, calls: &mut Vec<TokenStream>)
    where
        T: serde::Serialize,
    {
        let path_segments = path.to_vec();

        match rv {
            RuntimeValue::Constant(value) => {
                let value = serde_json::to_string(value)
                    .unwrap()
                    .parse::<TokenStream>()
                    .unwrap();

                calls.push(quote! {
                    .constant(
                        path![#(#path_segments),*],
                        serde_json::json!(#value)
                    )
                });
            }
            RuntimeValue::Runtime { default, .. } => {
                if let Some(default) = default {
                    let default = serde_json::to_string(default)
                        .unwrap()
                        .parse::<TokenStream>()
                        .unwrap();

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

    fn push_rv_mapper_option<T>(
        path: &[&str],
        rv: Option<&RuntimeValue<T>>,
        fields: &mut Vec<TokenStream>,
    ) {
        if let Some(rv) = rv {
            Self::push_rv_mapper(path, rv, fields);
        }
    }

    fn push_rv_mapper<T>(path: &[&str], rv: &RuntimeValue<T>, fields: &mut Vec<TokenStream>) {
        let path_segments = path.to_vec();

        if let RuntimeValue::Runtime { env, .. } = rv {
            fields.push(quote! {
                .field(path![#(#path_segments),*], #env)
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::context::{
        ResolvedContextAgent, ResolvedContextAgentModel, ResolvedContextRuntime,
        ResolvedContextTool, ResolvedContextToolA2a,
    };

    struct A2aCodeGenFixture;

    impl A2aCodeGenFixture {
        fn context() -> ResolvedContext {
            ResolvedContext {
                slug: "assistant".to_string(),
                agent_name: "assistant".to_string(),
                runtime: ResolvedContextRuntime {
                    default_tenant_id: RuntimeValue::constant("default".to_string()),
                },
                providers: vec![],
                agent: ResolvedContextAgent {
                    version: "0.1.0".to_string(),
                    description: None,
                    prompt: None,
                    capabilities: None,
                    capability_policy: None,
                    model: ResolvedContextAgentModel {
                        provider: RuntimeValue::constant("anthropic".to_string()),
                        name: RuntimeValue::constant("claude".to_string()),
                    },
                },
                blocks: HashMap::new(),
                tools: HashMap::from([(
                    "planner".to_string(),
                    ResolvedContextTool {
                        name: "planner".to_string(),
                        description: Some("Delegate planning.".to_string()),
                        enabled: RuntimeValue::Runtime {
                            env: "PLANNER_A2A_ENABLED".to_string(),
                            default: Some(true),
                            secret: false,
                        },
                        capabilities: vec!["a2a:planner".to_string()],
                        config: HashMap::new(),
                        kind: ResolvedContextToolKind::A2a(ResolvedContextToolA2a {
                            url: RuntimeValue::Runtime {
                                env: "PLANNER_A2A_URL".to_string(),
                                default: Some("https://planner.example.com".to_string()),
                                secret: false,
                            },
                            auth_token: Some(RuntimeValue::Runtime {
                                env: "PLANNER_A2A_TOKEN".to_string(),
                                default: None,
                                secret: true,
                            }),
                            headers: HashMap::from([(
                                "X-Agent".to_string(),
                                RuntimeValue::constant("assistant".to_string()),
                            )]),
                            tenant: ResolvedContextToolA2aTenant::Fixed {
                                id: RuntimeValue::Runtime {
                                    env: "PLANNER_A2A_TENANT".to_string(),
                                    default: Some("tenant-1".to_string()),
                                    secret: false,
                                },
                            },
                            timeout_secs: None,
                            default_accepted_output_modes: vec!["text/plain".to_string()],
                        }),
                    },
                )]),
                skills: HashMap::new(),
                http_server: None,
            }
        }

        fn compact(tokens: TokenStream) -> String {
            tokens
                .to_string()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
        }
    }

    #[test]
    fn loader_calls_lift_a2a_tool_into_a2a_agents_config() {
        let rendered =
            A2aCodeGenFixture::compact(A2aCodeGen::loader_calls(&A2aCodeGenFixture::context()));

        assert!(rendered.contains("path ! [\"a2a\" , \"agents\" , \"planner\" , \"url\"]"));
        assert!(rendered.contains("serde_json :: json ! (\"https://planner.example.com\")"));
        assert!(
            rendered
                .contains("path ! [\"a2a\" , \"agents\" , \"planner\" , \"tenant\" , \"policy\"]")
        );
        assert!(rendered.contains("serde_json :: json ! (\"fixed\")"));
        assert!(
            rendered.contains("path ! [\"a2a\" , \"agents\" , \"planner\" , \"tenant\" , \"id\"]")
        );
        assert!(rendered.contains("serde_json :: json ! (\"tenant-1\")"));
        assert!(rendered.contains("path ! [\"a2a\" , \"agents\" , \"planner\" , \"enabled\"]"));
        assert!(rendered.contains("serde_json :: json ! (true)"));
        assert!(
            rendered.contains("path ! [\"a2a\" , \"agents\" , \"planner\" , \"capabilities\"]")
        );
        assert!(rendered.contains("serde_json :: json ! ([\"a2a:planner\"])"));
    }

    #[test]
    fn mapper_fields_lift_runtime_a2a_values_into_a2a_agents_config() {
        let rendered =
            A2aCodeGenFixture::compact(A2aCodeGen::mapper_fields(&A2aCodeGenFixture::context()));

        assert!(rendered.contains(
            "path ! [\"a2a\" , \"agents\" , \"planner\" , \"url\"] , \"PLANNER_A2A_URL\""
        ));
        assert!(rendered.contains(
            "path ! [\"a2a\" , \"agents\" , \"planner\" , \"auth_token\"] , \"PLANNER_A2A_TOKEN\""
        ));
        assert!(rendered.contains(
            "path ! [\"a2a\" , \"agents\" , \"planner\" , \"tenant\" , \"id\"] , \"PLANNER_A2A_TENANT\""
        ));
        assert!(rendered.contains(
            "path ! [\"a2a\" , \"agents\" , \"planner\" , \"enabled\"] , \"PLANNER_A2A_ENABLED\""
        ));
        assert!(!rendered.contains("\"tool\" , \"planner\""));
    }
}
