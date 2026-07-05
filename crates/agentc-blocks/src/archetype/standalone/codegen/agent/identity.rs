// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use proc_macro2::TokenStream;
use quote::quote;

use agentc_compiler::generator::errors::GeneratorError;

use crate::{
    archetype::standalone::fields::FieldsSpec,
    context::{ResolvedContext, ResolvedContextAgentPromptMessageRole},
};

/// Generates the `AgentIdentity { ... }` literal wired into the agent builder.
pub struct IdentityCodeGen;

impl IdentityCodeGen {
    pub fn generate(
        ctx: &ResolvedContext,
        fields: &FieldsSpec,
    ) -> Result<TokenStream, GeneratorError> {
        let name = &ctx.agent_name;

        let provider = fields
            .config_accessor(&["agent", "model", "provider"])
            .map(|p| quote! { #p.clone() })
            .unwrap_or_else(|| {
                let v = ctx
                    .agent
                    .model
                    .provider
                    .default_value()
                    .cloned()
                    .unwrap_or_default();
                quote! { #v.to_string() }
            });

        let model = fields
            .config_accessor(&["agent", "model", "name"])
            .map(|p| quote! { #p.clone() })
            .unwrap_or_else(|| {
                let v = ctx
                    .agent
                    .model
                    .name
                    .default_value()
                    .cloned()
                    .unwrap_or_default();
                quote! { #v.to_string() }
            });

        let prompt = match &ctx.agent.prompt {
            None => quote! { PromptTemplate::default() },
            Some(messages) => {
                let parts = messages.iter().map(|message| {
                    let role = match message.role {
                        ResolvedContextAgentPromptMessageRole::System => quote! { Role::System },
                        ResolvedContextAgentPromptMessageRole::User => quote! { Role::User },
                        ResolvedContextAgentPromptMessageRole::Assistant => {
                            quote! { Role::Assistant }
                        }
                    };
                    let content = &message.content;

                    quote! { .with_part(#role, #content) }
                });

                quote! {
                    PromptTemplate::new()
                        #(#parts)*
                }
            }
        };

        let capabilities = fields
            .config_accessor(&["agent", "capabilities"])
            .map(|path| quote! { CapabilitySet::from(#path.clone()) })
            .unwrap_or_else(|| quote! { CapabilitySet::empty() });

        let capability_policy = ctx
            .agent
            .capability_policy
            .as_ref()
            .and_then(|_| fields.config_accessor(&["agent", "capability_policy"]))
            .map(|path| {
                quote! {
                    #path.parse::<CapabilityPolicy>().expect("invalid capability policy value")
                }
            })
            .unwrap_or_else(|| quote! { CapabilityPolicy::default() });

        Ok(quote! {
            AgentIdentity {
                name: #name.into(),
                provider: #provider.into(),
                model: #model.into(),
                prompt: #prompt,
                capabilities: #capabilities,
                capability_policy: #capability_policy,
            }
        })
    }
}
