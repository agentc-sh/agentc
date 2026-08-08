// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use quote::quote;

use agentc_compiler::generator::{
    blocks::fragment::{Fragment, FragmentBlock},
    context::GenerationContext,
    errors::GeneratorError,
    extension::{Contribution, ErasedContributionValue},
};

use crate::{
    config::sections::{
        block::ConfigSectionBlockBuilderExt,
        contribution::{ConfigSectionContribution, ConfigSections},
    },
    context::ResolvedContext,
    contributions::dependency::{
        CargoDependencies, CargoDependencyContribution, ExternalDependencyContribution,
    },
};

pub struct PubSubSection;

impl PubSubSection {
    pub const NAME: &'static str = "pubsub";

    pub fn block(id: &'static str) -> FragmentBlock<ResolvedContext> {
        FragmentBlock::<ResolvedContext>::builder()
            .id(id)
            .contribute_config_sections()
            .contribute(Contribution::<CargoDependencies>::strict("cargo::dependencies"))
            .build(Self)
    }

    fn section(&self) -> Result<ConfigSections, GeneratorError> {
        ConfigSections::from_entries([
            ConfigSectionContribution::new(Self::NAME)
                .uses(quote! {
                    use subway::{
                        Bus,
                        memory::InMemoryTransport,
                        redis::RedisTransport,
                    };
                })
                .types(quote! {
                    #[derive(Debug, Clone, Serialize, Deserialize)]
                    #[serde(tag = "kind", rename_all = "snake_case")]
                    pub enum PubSubConfig {
                        Memory {
                            capacity: usize,
                        },
                        Redis {
                            url: String,
                        },
                    }

                    impl PubSubConfig {
                        pub fn kind(&self) -> &str {
                            match self {
                                PubSubConfig::Memory { .. } => "memory",
                                PubSubConfig::Redis { .. } => "redis",
                            }
                        }

                        pub async fn build(&self) -> Result<Bus, subway::Error> {
                            match self {
                                PubSubConfig::Memory { capacity } => Ok(Bus::new(
                                    InMemoryTransport::with_capacity(*capacity),
                                )),
                                PubSubConfig::Redis { url } => Ok(Bus::new(
                                    RedisTransport::builder()
                                        .url(url.clone())
                                        .build()
                                        .await?,
                                )),
                            }
                        }
                    }

                    impl Default for PubSubConfig {
                        fn default() -> Self {
                            PubSubConfig::Memory {
                                capacity: 4096,
                            }
                        }
                    }
                })
                .fields(quote! {
                    pub pubsub: PubSubConfig,
                }),
        ])
        .map_err(|error| GeneratorError::unexpected(error.to_string()))
    }
}

impl Fragment<ResolvedContext> for PubSubSection {
    fn generate_contribution(
        &self,
        _ctx: &GenerationContext<ResolvedContext>,
        point: &str,
    ) -> Result<ErasedContributionValue, GeneratorError> {
        match point {
            "config::sections::use"
            | "config::sections::types"
            | "config::sections::fields"
            | "config::sections::loader"
            | "config::sections::mapper" => {
                Ok(ErasedContributionValue::new(self.section()?))
            }
            "cargo::dependencies" => Ok(ErasedContributionValue::new(
                CargoDependencies::from_entries([CargoDependencyContribution::external(
                    ExternalDependencyContribution::new("subway")
                        .git("https://github.com/wizrds/subway-rs.git")
                        .version("0.1.0")
                        .feature("redis"),
                )])
                .map_err(|error| GeneratorError::unexpected(error.to_string()))?,
            )),
            _ => Err(GeneratorError::unexpected(format!("Unknown extension point '{}'", point))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use serde_json::json;

    fn context() -> GenerationContext<ResolvedContext> {
        GenerationContext::new(
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
                "skills": {},
                "http_server": null
            }))
            .unwrap(),
        )
    }

    #[test]
    fn the_section_defines_pubsub_config_and_its_field() {
        let sections = PubSubSection
            .generate_contribution(&context(), "config::sections::types")
            .unwrap()
            .downcast::<ConfigSections>()
            .unwrap();
        let section = sections
            .get(&PubSubSection::NAME)
            .expect("pubsub section is contributed");

        assert!(section.types.as_str().contains("pub enum PubSubConfig"));
        assert!(
            section
                .fields
                .as_str()
                .contains("pub pubsub : PubSubConfig")
        );
    }

    #[test]
    fn contributes_the_subway_dependency() {
        let dependencies = PubSubSection
            .generate_contribution(&context(), "cargo::dependencies")
            .unwrap()
            .downcast::<CargoDependencies>()
            .unwrap();

        assert!(
            dependencies
                .get(&"subway")
                .is_some()
        );
    }
}
