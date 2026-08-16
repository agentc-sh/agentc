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

        let mut population = Vec::new();

        for skill in ctx.skills.values() {
            match &skill.kind {
                ResolvedContextSkillKind::Source(source) => {
                    let dir = &source.dir;

                    population.push(quote! {
                        skills = skills
                            .with_embedded(agentc_fs::embedded_dir!(#dir))
                            .await?;
                    });
                }

                ResolvedContextSkillKind::Content(content) => {
                    let skill_md = format!(
                        "---\nname: {}\ndescription: {}\n---\n{}",
                        skill.name, content.description, content.content,
                    );
                    let resources = content
                        .resources
                        .iter()
                        .map(|(rel, body)| quote! { (#rel, #body) })
                        .collect::<Vec<_>>();

                    population.push(quote! {
                        skills = skills.with_static(#skill_md, &[#(#resources),*])?;
                    });
                }
            }
        }

        let registrations = vec![quote! {
            let mut skills = SkillRegistryBuilder::default();

            #(#population)*

            let skills = skills.build();

            fs.mount_fs(agentc_skills::registry::SKILLS_ROOT, skills.fs())?;

            builder = builder.with_skill_registry(skills, MaterializationPolicy::OnDemand);
        }];

        Ok((imports, registrations))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::*;

    struct SkillsCodeGenFixture;

    impl SkillsCodeGenFixture {
        fn context(skills: Value) -> ResolvedContext {
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
                "tools": {},
                "skills": skills,
                "http_server": null
            }))
            .unwrap()
        }

        fn generate(skills: Value) -> String {
            SkillsCodeGen::generate(&Self::context(skills))
                .unwrap()
                .1
                .into_iter()
                .map(|tokens| tokens.to_string())
                .collect::<Vec<_>>()
                .join("\n")
        }
    }

    #[test]
    fn source_skills_embed_the_staged_directory() {
        let rendered = SkillsCodeGenFixture::generate(json!({
            "pdf": {
                "name": "pdf",
                "kind": {
                    "kind": "source",
                    "dir": "/artifacts/skills/pdf"
                }
            }
        }));

        assert!(rendered.contains("with_embedded"));
        assert!(rendered.contains("agentc_fs :: embedded_dir"));
        assert!(rendered.contains("/artifacts/skills/pdf"));
    }

    #[test]
    fn content_skills_seed_the_registry_with_static_content() {
        let rendered = SkillsCodeGenFixture::generate(json!({
            "notes": {
                "name": "notes",
                "kind": {
                    "kind": "content",
                    "description": "Note-taking instructions.",
                    "content": "Take structured notes.",
                    "resources": {
                        "references/template.md": "# Template"
                    }
                }
            }
        }));

        assert!(rendered.contains("with_static"));
        assert!(rendered.contains("name: notes"));
        assert!(rendered.contains("references/template.md"));
    }

    #[test]
    fn generated_registry_is_grafted_and_registered() {
        let rendered = SkillsCodeGenFixture::generate(json!({
            "notes": {
                "name": "notes",
                "kind": {
                    "kind": "content",
                    "description": "Note-taking instructions.",
                    "content": "Take structured notes.",
                    "resources": {}
                }
            }
        }));

        assert!(rendered.contains("let mut skills = SkillRegistryBuilder :: default"));
        assert!(rendered.contains(
            "fs . mount_fs (agentc_skills :: registry :: SKILLS_ROOT , skills . fs ())"
        ));
        assert!(rendered.contains(
            "with_skill_registry (skills , MaterializationPolicy :: OnDemand)"
        ));
    }
}
