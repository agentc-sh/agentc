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

use crate::context::{ResolvedContext, ResolvedContextHttpServerProtocolAgUi};

pub struct AgUiCodeGen {
    pub config: ResolvedContextHttpServerProtocolAgUi,
}

impl CodeGen<ResolvedContext> for AgUiCodeGen {
    fn generate_contribution(
        &self,
        _ctx: &GenerationContext<ResolvedContext>,
        point: &str,
    ) -> Result<TokenStream, GeneratorError> {
        match point {
            "agent::features" => Ok(quote! {
                "ag-ui"
            }),
            "server::routers" => {
                let config_path = &self.config.path;

                Ok(quote! {
                    builder = builder.with_router(
                        utoipa_axum::router::OpenApiRouter::new()
                            .nest(#config_path, agentc_protocol_ag_ui::router::router(state.clone()))
                    );
                })
            }
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
