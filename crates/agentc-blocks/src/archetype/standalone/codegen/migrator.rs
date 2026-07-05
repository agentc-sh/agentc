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
            use agentc_agent_react::migrations::all as react_migrations;

            #extra_use

            pub struct Migrator;

            #[async_trait::async_trait]
            impl MigratorTrait for Migrator {
                fn migrations() -> Vec<Box<dyn MigrationTrait>> {
                    [
                        domain_migrations(),
                        react_migrations(),
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
