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
        CargoDependencies, CargoDependencyContribution, CargoPatchContribution, CargoPatches,
        RuntimeDependencyContribution,
    },
};

pub struct NetworkSection;

impl NetworkSection {
    pub const NAME: &'static str = "network";

    pub fn block(id: &'static str) -> FragmentBlock<ResolvedContext> {
        FragmentBlock::<ResolvedContext>::builder()
            .id(id)
            .contribute_config_sections()
            .contribute(Contribution::<CargoDependencies>::strict("cargo::dependencies"))
            .contribute(Contribution::<CargoPatches>::strict("cargo::patches"))
            .build(Self)
    }

    fn section(&self) -> Result<ConfigSections, GeneratorError> {
        ConfigSections::from_entries([ConfigSectionContribution::new(Self::NAME)
            .uses(quote! {
                use agentc_http::client::{HttpClient, HttpClientBuilder};
            })
            .types(quote! {
                #[derive(Debug, Clone, Serialize, Deserialize, Default)]
                #[serde(default)]
                pub struct NetworkConfig {}

                impl NetworkConfig {
                    /// Returns an unbuilt client so each consumer can build on its own runtime.
                    pub fn builder(&self) -> HttpClientBuilder {
                        HttpClient::builder()
                    }
                }
            })
            .fields(quote! {
                pub network: NetworkConfig,
            })])
        .map_err(|error| GeneratorError::unexpected(error.to_string()))
    }
}

impl Fragment<ResolvedContext> for NetworkSection {
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
            | "config::sections::mapper" => Ok(ErasedContributionValue::new(self.section()?)),
            "cargo::dependencies" => Ok(ErasedContributionValue::new(
                CargoDependencies::from_entries([CargoDependencyContribution::runtime(
                    RuntimeDependencyContribution::new("agentc-http")
                        .default_features(false)
                        .feature("client"),
                )])
                .map_err(|error| GeneratorError::unexpected(error.to_string()))?,
            )),
            "cargo::patches" => Ok(ErasedContributionValue::new(
                CargoPatches::from_entries([CargoPatchContribution::runtime(
                    RuntimeDependencyContribution::new("agentc-http"),
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
    fn contributes_the_http_client_dependency_unconditionally() {
        let dependencies = NetworkSection
            .generate_contribution(&context(), "cargo::dependencies")
            .unwrap()
            .downcast::<CargoDependencies>()
            .unwrap();

        assert_eq!(dependencies.len(), 1);
        assert!(matches!(
            dependencies
                .get(&"agentc-http")
                .unwrap(),
            CargoDependencyContribution::Runtime(dependency)
                if dependency.default_features == Some(false)
                    && dependency.features.len() == 1
                    && dependency.features.contains("client")
        ));
    }

    #[test]
    fn the_section_defines_network_config_and_its_field() {
        let sections = NetworkSection
            .generate_contribution(&context(), "config::sections::types")
            .unwrap()
            .downcast::<ConfigSections>()
            .unwrap();
        let section = sections
            .get(&NetworkSection::NAME)
            .expect("network section is contributed");

        assert!(
            section
                .types
                .as_str()
                .contains("pub struct NetworkConfig")
        );
        assert!(
            section
                .fields
                .as_str()
                .contains("pub network : NetworkConfig")
        );
    }
}
