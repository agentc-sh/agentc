// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use proc_macro2::TokenStream;
use quote::quote;
use std::path::PathBuf;

use agentc_compiler::generator::{
    blocks::codegen::{CodeGen, CodeGenBlock},
    context::GenerationContext,
    errors::GeneratorError,
    extension::{
        Contribution, ErasedContributionValue, ExtensionRegistry, RenderedTokenStream, reducers,
    },
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

    pub fn block(id: &'static str) -> CodeGenBlock<ResolvedContext> {
        CodeGenBlock::<ResolvedContext>::builder()
            .id(id)
            .contribute_config_sections()
            .token_stream_extension_point("filesystem::mounts", reducers::concat)
            .token_stream_extension_point("filesystem::topology", reducers::last)
            .contribute(Contribution::<RenderedTokenStream>::strict("filesystem::topology"))
            .contribute(Contribution::<RenderedTokenStream>::strict("config::mods"))
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
        ConfigSections::from_entries([ConfigSectionContribution::new(Self::NAME).fields(quote! {
            pub filesystem: filesystem::ConfigFilesystem,
        })])
        .map_err(|error| GeneratorError::unexpected(error.to_string()))
    }
}

impl CodeGen<ResolvedContext> for FilesystemSection {
    fn generate_files(
        &self,
        _ctx: &GenerationContext<ResolvedContext>,
        registry: &ExtensionRegistry,
    ) -> Result<Vec<(PathBuf, TokenStream)>, GeneratorError> {
        let mounts = registry
            .get("filesystem::mounts")
            .and_then(|s| s.parse::<TokenStream>().ok());

        let topology = registry
            .get("filesystem::topology")
            .and_then(|s| s.parse::<TokenStream>().ok());

        Ok(vec![(
            "src/config/filesystem.rs".into(),
            quote! {
                use serde::{Deserialize, Serialize};

                use agentc_fs::fs::{Fs, FsBuilder};

                #[derive(Debug, Clone, Serialize, Deserialize, Default)]
                #[serde(default)]
                pub struct ConfigFilesystem {
                    pub binds: Vec<ConfigFilesystemBind>,
                }

                impl ConfigFilesystem {
                    pub fn builder(&self) -> Result<FsBuilder, agentc_fs::errors::Error> {
                        let mut builder = Fs::builder();

                        #mounts
                        #topology

                        for bind in &self.binds {
                            let host = agentc_fs::host::HostFs::builder()
                                .root(bind.root.clone())
                                .follow_symlinks(bind.follow_symlinks)
                                .build()?;

                            builder = if bind.readonly {
                                builder.mount(
                                    bind.path.clone(),
                                    agentc_fs::readonly::ReadOnlyFs::new(host),
                                )
                            } else {
                                builder.mount(bind.path.clone(), host)
                            };
                        }

                        Ok(builder)
                    }
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
            },
        )])
    }

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
            "config::mods" => Ok(ErasedContributionValue::new(RenderedTokenStream::from(quote! {
                pub mod filesystem;
            }))),
            "filesystem::topology" => Ok(ErasedContributionValue::new(RenderedTokenStream::from(
                self.topology_tokens(&ctx.filesystem),
            ))),
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
            .generate_contribution(&context(json!({ "mounts": [] })), "cargo::dependencies")
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
    fn the_section_emits_its_own_file_with_the_config_struct_and_builder() {
        let source = FilesystemSection
            .generate_files(&context(json!({ "mounts": [] })), &ExtensionRegistry::empty())
            .unwrap()
            .into_iter()
            .find(|(path, _)| path == &PathBuf::from("src/config/filesystem.rs"))
            .expect("the filesystem section generates its own config file")
            .1
            .to_string();

        assert!(source.contains("pub struct ConfigFilesystem"));
        assert!(source.contains("pub binds : Vec < ConfigFilesystemBind >"));
        assert!(source.contains("impl ConfigFilesystem"));
        assert!(source.contains("pub fn builder (& self)"));
    }

    #[test]
    fn the_section_declares_a_module_qualified_config_field() {
        let sections = FilesystemSection
            .generate_contribution(&context(json!({ "mounts": [] })), "config::sections::fields")
            .unwrap()
            .downcast::<ConfigSections>()
            .unwrap();

        assert!(
            sections
                .get(&FilesystemSection::NAME)
                .expect("filesystem section is contributed")
                .fields
                .as_str()
                .contains("pub filesystem : filesystem :: ConfigFilesystem")
        );
    }

    #[test]
    fn contributes_its_module_declaration() {
        let module = FilesystemSection
            .generate_contribution(&context(json!({ "mounts": [] })), "config::mods")
            .unwrap()
            .downcast::<RenderedTokenStream>()
            .unwrap()
            .as_str()
            .to_string();

        assert!(module.contains("pub mod filesystem ;"));
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
            .downcast::<RenderedTokenStream>()
            .unwrap()
            .as_str()
            .to_string();

        assert!(topology.contains(
            r#"builder = builder . mount ("/" , agentc_fs :: memory :: MemoryFs :: new ())"#
        ));
        assert!(topology.contains(
            r#"agentc_fs :: readonly :: ReadOnlyFs :: new (agentc_fs :: host :: HostFs :: builder () . root ("/var/lib/agent/data")"#
        ));
    }
}
