// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::path::PathBuf;

use proc_macro2::TokenStream;
use serde::Serialize;

use crate::generator::{
    context::GenerationContext, errors::GeneratorError, extension::ExtensionRegistry,
};

/// Implemented by types that produce Rust source code via syn or quote.
///
/// # Example
///
/// ```rust,ignore
/// struct MyGen;
///
/// impl CodeGen<MyConfig> for MyGen {
///     fn generate_files(
///         &self,
///         ctx: &GenerationContext<MyConfig>,
///         registry: &ExtensionRegistry,
///     ) -> Result<Vec<(PathBuf, TokenStream)>, GeneratorError> {
///         let name = format_ident!("{}", ctx.struct_name);
///         let stream = quote! {
///             pub struct #name {
///                 client: Box<dyn LlmClient>,
///             }
///         };
///
///         Ok(vec![("src/agent.rs".into(), stream)])
///     }
/// }
/// ```
pub trait CodeGen<T>: Send + Sync
where
    T: Serialize + Send + Sync,
{
    /// Generate file contents as [`TokenStream`](proc_macro2::TokenStream) entries.
    fn generate_files(
        &self,
        ctx: &GenerationContext<T>,
        registry: &ExtensionRegistry,
    ) -> Result<Vec<(PathBuf, TokenStream)>, GeneratorError>;

    /// Generate a contribution for a named extension point as a [`TokenStream`](proc_macro2::TokenStream).
    fn generate_contribution(
        &self,
        ctx: &GenerationContext<T>,
        point: &str,
    ) -> Result<TokenStream, GeneratorError> {
        let _ = (ctx, point);
        Ok(TokenStream::new())
    }
}
