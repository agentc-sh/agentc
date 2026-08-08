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
    fn generate_contribution(
        &self,
        _ctx: &GenerationContext<ResolvedContext>,
        point: &str,
    ) -> Result<TokenStream, GeneratorError> {
        match point {
            "main::modules" => Ok(quote! {
                mod migrator;
            }),
            _ => Err(GeneratorError::unexpected(format!("Unknown extension point '{}'", point))),
        }
    }

    fn generate_files(
        &self,
        _ctx: &GenerationContext<ResolvedContext>,
        _registry: &ExtensionRegistry,
    ) -> Result<Vec<(PathBuf, TokenStream)>, GeneratorError> {
        let source = quote! {
            use sea_orm_migration::prelude::*;

            use agentc_agent_react::migrations::all as react_migrations;
            use agentc_domain_sql::migrations::all as domain_migrations;

            pub struct Migrator;

            #[async_trait::async_trait]
            impl MigratorTrait for Migrator {
                fn migrations() -> Vec<Box<dyn MigrationTrait>> {
                    [
                        domain_migrations(),
                        react_migrations(),
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
    fn migrator_includes_domain_and_react_migrations() {
        let files = MigratorCodeGen
            .generate_files(&context(), &ExtensionRegistry::empty())
            .unwrap();
        let source = files[0].1.to_string();

        assert!(source.contains("domain_migrations"));
        assert!(source.contains("react_migrations"));
    }

    #[test]
    fn contributes_the_main_migrator_module() {
        let source = MigratorCodeGen
            .generate_contribution(&context(), "main::modules")
            .unwrap()
            .to_string();

        assert!(source.contains("mod migrator ;"));
    }
}
