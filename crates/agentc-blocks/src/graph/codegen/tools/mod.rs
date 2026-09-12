// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

pub mod bash;
pub mod javascript;
pub mod python;

use proc_macro2::TokenStream;
use quote::quote;

use agentc_compiler::generator::errors::GeneratorError;

use crate::{
    config::fields::FieldsSpec,
    context::ResolvedContext,
    contributions::import::{ImportContribution, Imports},
    graph::codegen::tools::{
        bash::BashTools,
        javascript::JavascriptTools,
        python::{CPythonTools, RustPythonTools},
    },
};

/// Code generation for a single tool kind.
///
/// Implemented once per tool kind so each kind fully owns how its tools are grouped,
/// configured, and registered, independent of the others. A kind that is not present
/// in the context reports no imports, no feature, and no registrations.
pub trait ToolCodeGen {
    /// The imports required by this kind's registrations, empty when no tools of this kind are
    /// present.
    fn imports(&self) -> Vec<ImportContribution>;

    /// The cargo feature enabled when tools of this kind are present, if any.
    fn feature(&self) -> Option<&'static str>;

    /// The `builder = builder.with_tool(...)` statements for every tool of this kind.
    fn registrations(&self, fields: &FieldsSpec) -> Result<Vec<TokenStream>, GeneratorError>;
}

/// Aggregates tool code generation across every supported tool kind.
pub struct ToolsCodeGen;

impl ToolsCodeGen {
    fn generators(ctx: &ResolvedContext) -> [Box<dyn ToolCodeGen + '_>; 4] {
        [
            Box::new(JavascriptTools(ctx)),
            Box::new(BashTools(ctx)),
            Box::new(RustPythonTools(ctx)),
            Box::new(CPythonTools(ctx)),
        ]
    }

    pub fn imports(ctx: &ResolvedContext) -> Result<Imports, GeneratorError> {
        Imports::from_entries(
            Self::generators(ctx)
                .iter()
                .flat_map(|generator| generator.imports()),
        )
        .map_err(|error| GeneratorError::unexpected(error.to_string()))
    }

    pub fn registrations(
        ctx: &ResolvedContext,
        fields: &FieldsSpec,
    ) -> Result<Vec<TokenStream>, GeneratorError> {
        let mut registrations = Vec::new();

        for generator in Self::generators(ctx) {
            registrations.extend(generator.registrations(fields)?);
        }

        Ok(registrations)
    }

    /// The comma-separated cargo feature names for every tool kind present, as
    /// contributed to the `tools::features` extension point.
    pub fn features(ctx: &ResolvedContext) -> TokenStream {
        let features = Self::generators(ctx)
            .iter()
            .filter_map(|g| g.feature())
            .map(|f| quote! { #f })
            .collect::<Vec<_>>();

        quote! { #(#features),* }
    }
}
