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

pub struct EntrypointCodeGen;

impl CodeGen<ResolvedContext> for EntrypointCodeGen {
    fn generate_files(
        &self,
        ctx: &GenerationContext<ResolvedContext>,
        registry: &ExtensionRegistry,
    ) -> Result<Vec<(PathBuf, TokenStream)>, GeneratorError> {
        let agent_name = &ctx.agent_name;
        let extra_modules = registry
            .get("main::modules")
            .and_then(|s| s.parse::<TokenStream>().ok());

        let source = quote! {
            mod agent;
            mod cli;
            mod config;
            mod migrator;

            #extra_modules

            use anyhow::Result;
            use agentc_telemetry::bootstrap;

            fn main() -> Result<()> {
                bootstrap(#agent_name, |telemetry| async move {
                    cli::run(telemetry).await
                })
            }
        };

        Ok(vec![("src/main.rs".into(), source)])
    }
}
