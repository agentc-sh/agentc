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

pub struct FilesystemSection;

impl FilesystemSection {
    pub const NAME: &'static str = "filesystem";

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
                use agentc_fs::{Fs, fs::FsBuilder, memory::MemoryFs};
            })
            .types(quote! {
                #[derive(Debug, Clone, Serialize, Deserialize, Default)]
                #[serde(default)]
                pub struct ConfigFilesystem {}

                impl ConfigFilesystem {
                    pub fn builder(&self) -> FsBuilder {
                        Fs::builder().mount("/", MemoryFs::new())
                    }
                }
            })
            .fields(quote! {
                pub filesystem: ConfigFilesystem,
            })])
        .map_err(|error| GeneratorError::unexpected(error.to_string()))
    }
}

impl Fragment<ResolvedContext> for FilesystemSection {
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
                    RuntimeDependencyContribution::new("agentc-fs")
                        .default_features(false)
                        .feature("embedded"),
                )])
                .map_err(|error| GeneratorError::unexpected(error.to_string()))?,
            )),
            "cargo::patches" => Ok(ErasedContributionValue::new(
                CargoPatches::from_entries([CargoPatchContribution::runtime(
                    RuntimeDependencyContribution::new("agentc-fs"),
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
    fn contributes_the_fs_dependency_unconditionally() {
        let dependencies = FilesystemSection
            .generate_contribution(&context(), "cargo::dependencies")
            .unwrap()
            .downcast::<CargoDependencies>()
            .unwrap();

        assert_eq!(dependencies.len(), 1);
        assert!(matches!(
            dependencies
                .get(&"agentc-fs")
                .unwrap(),
            CargoDependencyContribution::Runtime(dependency)
                if dependency.default_features == Some(false)
                    && dependency.features.len() == 1
                    && dependency.features.contains("embedded")
        ));
    }

    #[test]
    fn the_section_defines_filesystem_config_and_its_field() {
        let sections = FilesystemSection
            .generate_contribution(&context(), "config::sections::types")
            .unwrap()
            .downcast::<ConfigSections>()
            .unwrap();
        let section = sections
            .get(&FilesystemSection::NAME)
            .expect("filesystem section is contributed");

        assert!(
            section
                .types
                .as_str()
                .contains("pub struct ConfigFilesystem")
        );
        assert!(
            section
                .types
                .as_str()
                .contains("MemoryFs")
        );
        assert!(
            section
                .fields
                .as_str()
                .contains("pub filesystem : ConfigFilesystem")
        );
    }
}
