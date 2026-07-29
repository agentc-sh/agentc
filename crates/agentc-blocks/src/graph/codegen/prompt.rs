// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use proc_macro2::TokenStream;
use quote::quote;

use agentc_compiler::generator::errors::GeneratorError;

use crate::context::{
    ResolvedContext, ResolvedContextAgentPromptMessage, ResolvedContextAgentPromptMessageRole,
    ResolvedContextAgentPromptSource,
};

/// Generates the `PromptSource` argument wired into `with_prompt_source`.
pub struct PromptSourceCodeGen;

impl PromptSourceCodeGen {
    pub fn generate(ctx: &ResolvedContext) -> Result<TokenStream, GeneratorError> {
        Ok(match &ctx.agent.prompt {
            None => Self::constant(&[]),
            Some(ResolvedContextAgentPromptSource::Constant { messages }) => {
                Self::constant(messages)
            }
        })
    }

    fn constant(messages: &[ResolvedContextAgentPromptMessage]) -> TokenStream {
        if messages.is_empty() {
            return quote! { ConstantPromptSource::new(PromptTemplate::default()) };
        }

        let parts = messages.iter().map(|message| {
            let role = match message.role {
                ResolvedContextAgentPromptMessageRole::System => quote! { Role::System },
                ResolvedContextAgentPromptMessageRole::User => quote! { Role::User },
                ResolvedContextAgentPromptMessageRole::Assistant => quote! { Role::Assistant },
            };
            let content = &message.content;

            quote! { .with_part(#role, #content) }
        });

        quote! {
            ConstantPromptSource::new(
                PromptTemplate::new()
                    #(#parts)*
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn constant_source_generates_prompt_template() {
        let rendered = PromptSourceCodeGen::generate(
            &serde_json::from_value::<ResolvedContext>(json!({
                "slug": "assistant",
                "agent_name": "assistant",
                "runtime": { "default_tenant_id": "default" },
                "providers": [],
                "agent": {
                    "version": "0.1.0",
                    "description": null,
                    "prompt": {
                        "constant": {
                            "messages": [{
                                "role": "system",
                                "content": "hi {{ agent_name }}"
                            }]
                        }
                    },
                    "capabilities": null,
                    "capability_policy": null,
                    "model": { "provider": "anthropic", "name": "claude" }
                },
                "blocks": {},
                "tools": {},
                "skills": {},
                "http_server": null
            }))
            .unwrap(),
        )
        .unwrap()
        .to_string();

        assert!(rendered.contains("ConstantPromptSource :: new"));
        assert!(rendered.contains("PromptTemplate :: new"));
        assert!(rendered.contains("Role :: System"));
        assert!(rendered.contains("hi {{ agent_name }}"));
    }
}
