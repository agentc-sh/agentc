// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use proc_macro2::TokenStream;
use quote::quote;

use agentc_compiler::generator::errors::GeneratorError;

use crate::context::{ResolvedContext, ResolvedContextSkillKind};

/// Generates the skill registry construction and registration for the agent.
pub struct SkillsCodeGen;

impl SkillsCodeGen {
    pub fn generate(
        ctx: &ResolvedContext,
    ) -> Result<(Vec<TokenStream>, Vec<TokenStream>), GeneratorError> {
        if ctx.skills.is_empty() {
            return Ok((vec![], vec![]));
        }

        let imports = vec![quote! {
            use agentc_skills::{
                registry::SkillRegistryBuilder,
                builder::AgentBuilderSkillsExt,
                tools::run::MaterializationPolicy,
            };
        }];

        let mut with_static_calls = Vec::new();

        for skill in ctx.skills.values() {
            match &skill.kind {
                ResolvedContextSkillKind::Source(s) => {
                    let skill_md_path = &s.skill_md_path;
                    let resources = s
                        .resources
                        .iter()
                        .map(|(rel, abs)| quote! { (#rel, include_str!(#abs)) })
                        .collect::<Vec<_>>();

                    with_static_calls.push(quote! {
                        .with_static(include_str!(#skill_md_path), &[#(#resources),*])?
                    });
                }

                ResolvedContextSkillKind::Content(c) => {
                    let skill_md = format!(
                        "---\nname: {}\ndescription: {}\n---\n{}",
                        skill.name, c.description, c.content,
                    );
                    let resources = c
                        .resources
                        .iter()
                        .map(|(rel, content)| quote! { (#rel, #content) })
                        .collect::<Vec<_>>();

                    with_static_calls.push(quote! {
                        .with_static(#skill_md, &[#(#resources),*])?
                    });
                }
            }
        }

        let registrations = vec![quote! {
            builder = builder.with_skill_registry(
                SkillRegistryBuilder::default()
                    #(#with_static_calls)*
                    .build(),
                MaterializationPolicy::OnDemand,
            );
        }];

        Ok((imports, registrations))
    }
}
