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

pub struct DatabaseSection;

impl DatabaseSection {
    pub const NAME: &'static str = "database";

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
                use agentc_database::{
                    Database,
                    database::DatabaseOptions,
                    errors::DatabaseError,
                };

                use crate::migrator::Migrator;
            })
            .types(quote! {
                #[derive(Debug, Clone, Serialize, Deserialize)]
                #[serde(default)]
                pub struct DatabaseConfig {
                    pub primary: String,
                    pub replicas: Vec<String>,
                    pub options: DatabaseOptions,
                    pub auto_migrate: bool,
                }

                impl DatabaseConfig {
                    pub async fn build(
                        &self,
                        run_migrations: bool,
                    ) -> Result<Database, DatabaseError> {
                        let db = Database::builder()
                            .with_primary(self.primary.clone())
                            .with_replicas(self.replicas.clone())
                            .with_options(self.options.clone())
                            .build()
                            .await?;

                        if run_migrations {
                            db.run_migrations::<Migrator>().await?;
                        }

                        Ok(db)
                    }
                }

                impl Default for DatabaseConfig {
                    fn default() -> Self {
                        DatabaseConfig {
                            primary: "sqlite://database.db?mode=rwc".to_string(),
                            replicas: vec![],
                            options: DatabaseOptions::default(),
                            auto_migrate: true,
                        }
                    }
                }
            })
            .fields(quote! {
                pub database: DatabaseConfig,
            })])
        .map_err(|error| GeneratorError::unexpected(error.to_string()))
    }
}

impl Fragment<ResolvedContext> for DatabaseSection {
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
                    RuntimeDependencyContribution::new("agentc-database"),
                )])
                .map_err(|error| GeneratorError::unexpected(error.to_string()))?,
            )),
            "cargo::patches" => Ok(ErasedContributionValue::new(
                CargoPatches::from_entries([CargoPatchContribution::runtime(
                    RuntimeDependencyContribution::new("agentc-database"),
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
    fn the_section_defines_database_config_and_its_field() {
        let sections = DatabaseSection
            .generate_contribution(&context(), "config::sections::types")
            .unwrap()
            .downcast::<ConfigSections>()
            .unwrap();
        let section = sections
            .get(&DatabaseSection::NAME)
            .expect("database section is contributed");

        assert!(
            section
                .types
                .as_str()
                .contains("pub struct DatabaseConfig")
        );
        assert!(
            section
                .types
                .as_str()
                .contains("auto_migrate : true")
        );
        assert!(
            section
                .fields
                .as_str()
                .contains("pub database : DatabaseConfig")
        );
    }

    #[test]
    fn contributes_the_database_runtime_dependency() {
        let dependencies = DatabaseSection
            .generate_contribution(&context(), "cargo::dependencies")
            .unwrap()
            .downcast::<CargoDependencies>()
            .unwrap();

        assert_eq!(dependencies.len(), 1);
        assert!(
            dependencies
                .get(&"agentc-database")
                .is_some()
        );
    }
}
