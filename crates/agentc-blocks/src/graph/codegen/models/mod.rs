// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

pub mod anthropic;
pub mod gemini;
pub mod ollama;
pub mod openai;
pub mod openrouter;
pub mod params;
pub mod xai;

use proc_macro2::TokenStream;

use agentc_compiler::generator::errors::GeneratorError;

use crate::{
    context::{ResolvedContext, ResolvedContextProvider},
    fields::FieldsSpec,
};

/// Model-registry code generation for a single provider.
///
/// Implemented once per provider on that provider's resolved context struct so each
/// provider fully owns the shape of its own registration: its imports, its config
/// struct, its model constraints, and its inference parameters. Nothing about one
/// provider's config or params is assumed to match another's, so a new provider with
/// a different shape only needs its own impl.
pub trait ModelCodeGen {
    /// The `use` statement bringing this provider's factory and config types into scope.
    fn imports(&self) -> TokenStream;

    /// The full `ModelRegistry` builder call chain that registers this provider.
    fn registration(&self, fields: &FieldsSpec) -> TokenStream;
}

/// Aggregates model-registry code across every provider present in the context.
pub struct ModelRegistryCodeGen;

impl ModelRegistryCodeGen {
    pub fn generate(
        ctx: &ResolvedContext,
        fields: &FieldsSpec,
    ) -> Result<(Vec<TokenStream>, Vec<TokenStream>), GeneratorError> {
        let mut imports = Vec::new();
        let mut registrations = Vec::new();

        for provider in &ctx.providers {
            let model: &dyn ModelCodeGen = match provider {
                ResolvedContextProvider::Anthropic(p) => p,
                ResolvedContextProvider::OpenAi(p) => p,
                ResolvedContextProvider::Ollama(p) => p,
                ResolvedContextProvider::OpenRouter(p) => p,
                ResolvedContextProvider::Xai(p) => p,
                ResolvedContextProvider::Gemini(p) => p,
            };

            imports.push(model.imports());
            registrations.push(model.registration(fields));
        }

        Ok((imports, registrations))
    }
}
