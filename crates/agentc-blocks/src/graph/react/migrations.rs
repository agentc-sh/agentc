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

/// Contributes ReAct's own migrations into the generic `Migrator` that standalone
/// generates, through the `migrator::use` and `migrator::migrations` extension points.
pub struct ReActMigrationsCodeGen;

impl CodeGen<ResolvedContext> for ReActMigrationsCodeGen {
    fn generate_contribution(
        &self,
        _ctx: &GenerationContext<ResolvedContext>,
        point: &str,
    ) -> Result<TokenStream, GeneratorError> {
        match point {
            "migrator::use" => Ok(quote! {
                use agentc_agent_react::migrations::all as react_migrations;
            }),
            "migrator::migrations" => Ok(quote! {
                react_migrations(),
            }),
            _ => Err(GeneratorError::unexpected(format!("Unknown extension point '{}'", point))),
        }
    }

    fn generate_files(
        &self,
        _ctx: &GenerationContext<ResolvedContext>,
        _registry: &ExtensionRegistry,
    ) -> Result<Vec<(PathBuf, TokenStream)>, GeneratorError> {
        Ok(vec![])
    }
}
