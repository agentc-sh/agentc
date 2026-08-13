// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use proc_macro2::TokenStream;
use quote::quote;

use agentc_compiler::generator::{
    blocks::fragment::{Fragment, FragmentBlock},
    context::GenerationContext,
    errors::GeneratorError,
    extension::{Contribution, ErasedContributionValue, reducers},
};

use crate::{
    config::sections::{
        block::ConfigSectionBlockBuilderExt,
        contribution::{ConfigSectionContribution, ConfigSections},
    },
    context::{ResolvedContext, ResolvedContextFilesystem},
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
            .extension_point("filesystem::mounts", reducers::concat)
            .extension_point("filesystem::topology", reducers::last)
            .contribute(Contribution::<String>::strict("filesystem::topology"))
            .contribute(Contribution::<CargoDependencies>::strict("cargo::dependencies"))
            .contribute(Contribution::<CargoPatches>::strict("cargo::patches"))
            .build(Self)
    }

    fn topology_tokens(&self, filesystem: &ResolvedContextFilesystem) -> TokenStream {
        let mounts = filesystem
            .mounts
            .iter()
            .map(|mount| {
                let path = &mount.path;
                let backend = mount.backend.tokens();

                quote! {
                    builder = builder.mount(#path, #backend);
                }
            })
            .collect::<Vec<_>>();

        quote! { #(#mounts)* }
    }

    fn section(&self) -> Result<ConfigSections, GeneratorError> {
        ConfigSections::from_entries([ConfigSectionContribution::new(Self::NAME)
            .uses(quote! {
                use agentc_fs::{
                    fs::{Fs, FsBuilder},
                    host::HostFs,
                    memory::MemoryFs,
                    overlay::OverlayFs,
                    readonly::ReadOnlyFs,
                };
            })
            .types(quote! {
                #[derive(Debug, Clone, Serialize, Deserialize, Default)]
                #[serde(default)]
                pub struct ConfigFilesystem {
                    pub binds: Vec<ConfigFilesystemBind>,
                }

                #[derive(Debug, Clone, Serialize, Deserialize)]
                pub struct ConfigFilesystemBind {
                    pub path: String,
                    pub root: String,
                    #[serde(default)]
                    pub readonly: bool,
                    #[serde(default)]
                    pub follow_symlinks: bool,
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
        ctx: &GenerationContext<ResolvedContext>,
        point: &str,
    ) -> Result<ErasedContributionValue, GeneratorError> {
        match point {
            "config::sections::use"
            | "config::sections::types"
            | "config::sections::fields"
            | "config::sections::loader"
            | "config::sections::mapper" => Ok(ErasedContributionValue::new(self.section()?)),
            "filesystem::topology" => Ok(ErasedContributionValue::new(
                self.topology_tokens(&ctx.filesystem)
                    .to_string(),
            )),
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

    fn context(filesystem: serde_json::Value) -> GenerationContext<ResolvedContext> {
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
                "http_server": null,
                "filesystem": filesystem
            }))
            .unwrap(),
        )
    }

    #[test]
    fn contributes_the_fs_dependency_with_the_embedded_feature() {
        let dependencies = FilesystemSection
            .generate_contribution(
                &context(json!({ "mounts": [] })),
                "cargo::dependencies",
            )
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
    fn the_section_defines_filesystem_config_with_only_binds_and_no_builder_impl() {
        let sections = FilesystemSection
            .generate_contribution(
                &context(json!({ "mounts": [] })),
                "config::sections::types",
            )
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
                .contains("pub binds : Vec < ConfigFilesystemBind >")
        );
        assert!(
            !section
                .types
                .as_str()
                .contains("fn builder")
        );
        assert!(
            section
                .fields
                .as_str()
                .contains("pub filesystem : ConfigFilesystem")
        );
    }

    #[test]
    fn topology_emits_one_mount_statement_per_configured_mount() {
        let topology = FilesystemSection
            .generate_contribution(
                &context(json!({
                    "mounts": [
                        { "path": "/", "backend": { "Memory": null } },
                        {
                            "path": "/data",
                            "backend": {
                                "ReadOnly": {
                                    "inner": {
                                        "Host": {
                                            "root": "/var/lib/agent/data",
                                            "follow_symlinks": false
                                        }
                                    }
                                }
                            }
                        }
                    ]
                })),
                "filesystem::topology",
            )
            .unwrap()
            .downcast::<String>()
            .unwrap();

        assert!(topology.contains(r#"builder = builder . mount ("/" , MemoryFs :: new ())"#));
        assert!(
            topology.contains(
                r#"ReadOnlyFs :: new (HostFs :: builder () . root ("/var/lib/agent/data")"#
            )
        );
    }
}
