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
    context::ResolvedContext,
    fields::FieldsSpec,
    graph::codegen::tools::{
        bash::BashTools, javascript::JavascriptTools, python::EmbeddedPythonTools,
    },
};

/// Code generation for a single tool kind.
///
/// Implemented once per tool kind so each kind fully owns how its tools are grouped,
/// configured, and registered, independent of the others. A kind that is not present
/// in the context reports no imports, no feature, and no registrations.
pub trait ToolCodeGen {
    /// The `use` statement required by this kind's registrations, or `None` when no
    /// tools of this kind are present.
    fn imports(&self) -> Option<TokenStream>;

    /// The cargo feature enabled when tools of this kind are present, if any.
    fn feature(&self) -> Option<&'static str>;

    /// The `builder = builder.with_tool(...)` statements for every tool of this kind.
    fn registrations(&self, fields: &FieldsSpec) -> Result<Vec<TokenStream>, GeneratorError>;
}

/// Aggregates tool code generation across every supported tool kind.
pub struct ToolsCodeGen;

impl ToolsCodeGen {
    fn generators(ctx: &ResolvedContext) -> [Box<dyn ToolCodeGen + '_>; 3] {
        [
            Box::new(JavascriptTools(ctx)),
            Box::new(BashTools(ctx)),
            Box::new(EmbeddedPythonTools(ctx)),
        ]
    }

    pub fn generate(
        ctx: &ResolvedContext,
        fields: &FieldsSpec,
    ) -> Result<(Vec<TokenStream>, Vec<TokenStream>), GeneratorError> {
        let mut imports = Vec::new();
        let mut registrations = Vec::new();

        for generator in Self::generators(ctx) {
            if let Some(import) = generator.imports() {
                imports.push(import);
            }

            registrations.extend(generator.registrations(fields)?);
        }

        Ok((imports, registrations))
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
