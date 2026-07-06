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

/// Contributes the `agentc-agent-react` dependency and patch entries into the generic
/// Cargo project shell that standalone generates, through the `cargo::dependencies`
/// and `cargo::patches` extension points.
///
/// ReAct knows its own "ag-ui" cargo feature and enables it directly when an AG-UI
/// protocol is configured, rather than requiring AG-UI to know ReAct's crate name.
pub struct ReActCargoCodeGen {
    pub has_ag_ui: bool,
}

impl CodeGen<ResolvedContext> for ReActCargoCodeGen {
    fn generate_contribution(
        &self,
        _ctx: &GenerationContext<ResolvedContext>,
        point: &str,
    ) -> Result<TokenStream, GeneratorError> {
        let version = env!("CARGO_PKG_VERSION");

        match point {
            "cargo::dependencies" => {
                let features = if self.has_ag_ui {
                    quote! { "ag-ui" }
                } else {
                    quote! {}
                };

                Ok(quote! {
                    agentc-agent-react = { version = #version, features = [#features] }
                })
            }
            "cargo::patches" => Ok(quote! {
                agentc-agent-react = { path = "../runtime/agentc-agent-react" }
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
