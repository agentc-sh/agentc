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

pub struct CliMigrateCodeGen;

impl CodeGen<ResolvedContext> for CliMigrateCodeGen {
    fn generate_files(
        &self,
        _ctx: &GenerationContext<ResolvedContext>,
        _registry: &ExtensionRegistry,
    ) -> Result<Vec<(PathBuf, TokenStream)>, GeneratorError> {
        let source = quote! {
            use anyhow::Result;

            use crate::config::Config;

            pub async fn migrate() -> Result<()> {
                let config = Config::load().await?;

                config.database.build(true).await?;

                Ok(())
            }
        };

        Ok(vec![("src/cli/migrate.rs".into(), source)])
    }
}
