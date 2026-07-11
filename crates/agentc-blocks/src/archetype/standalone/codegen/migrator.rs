// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use proc_macro2::TokenStream;
use quote::quote;
use std::path::PathBuf;

use agentc_compiler::generator::{
    blocks::codegen::CodeGen, context::GenerationContext, errors::GeneratorError,
    extension::ExtensionRegistry,
};

use crate::context::ResolvedContext;

pub struct MigratorCodeGen;

impl CodeGen<ResolvedContext> for MigratorCodeGen {
    fn generate_files(
        &self,
        _ctx: &GenerationContext<ResolvedContext>,
        registry: &ExtensionRegistry,
    ) -> Result<Vec<(PathBuf, TokenStream)>, GeneratorError> {
        let extra_use = registry
            .get("migrator::use")
            .and_then(|s| s.parse::<TokenStream>().ok());

        let extra_migrations = registry
            .get("migrator::migrations")
            .and_then(|s| s.parse::<TokenStream>().ok());

        let source = quote! {
            use sea_orm_migration::prelude::*;

            use agentc_domain_sql::migrations::all as domain_migrations;

            #extra_use

            pub struct Migrator;

            #[async_trait::async_trait]
            impl MigratorTrait for Migrator {
                fn migrations() -> Vec<Box<dyn MigrationTrait>> {
                    [
                        domain_migrations(),
                        #extra_migrations
                    ]
                    .into_iter()
                    .flatten()
                    .collect()
                }
            }
        };

        Ok(vec![("src/migrator.rs".into(), source)])
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use agentc_compiler::generator::extension::{
        ErasedContributionValue, StringExtensionPoint, reducers,
    };
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
    fn generic_migrator_has_no_react_references() {
        let files = MigratorCodeGen
            .generate_files(&context(), &ExtensionRegistry::empty())
            .unwrap();
        let source = files[0].1.to_string();

        assert!(source.contains("domain_migrations"));
        assert!(!source.contains("react"));
        assert!(!source.contains("agentc_agent_react"));
    }

    #[test]
    fn extra_migrations_extension_point_is_still_honored() {
        let files = MigratorCodeGen
            .generate_files(
                &context(),
                &ExtensionRegistry::resolve(
                    vec![Box::new(StringExtensionPoint::new(
                        "migrator::migrations",
                        reducers::concat,
                    ))],
                    HashMap::from([(
                        "migrator::migrations".to_string(),
                        vec![ErasedContributionValue::new(
                            "fake_migrations(),".to_string(),
                        )],
                    )]),
                )
                .unwrap(),
            )
            .unwrap();
        let source = files[0].1.to_string();

        assert!(source.contains("fake_migrations"));
    }
}
