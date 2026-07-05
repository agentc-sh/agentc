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

pub struct BuildScriptCodeGen;

impl CodeGen<ResolvedContext> for BuildScriptCodeGen {
    fn generate_files(
        &self,
        ctx: &GenerationContext<ResolvedContext>,
        _registry: &ExtensionRegistry,
    ) -> Result<Vec<(PathBuf, TokenStream)>, GeneratorError> {
        let agent_version = ctx.agent.version.clone();
        let source = quote! {
            fn main() {
                println!("cargo:rustc-env=CARGO_PKG_VERSION={}", #agent_version);
            }

        };

        Ok(vec![("build.rs".into(), source)])
    }
}
