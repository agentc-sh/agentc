// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use proc_macro2::TokenStream;
use quote::quote;
use std::path::PathBuf;

use agentc_compiler::generator::{
    blocks::codegen::CodeGen,
    context::GenerationContext,
    errors::GeneratorError,
    extension::{ErasedContributionValue, ExtensionRegistry, RenderedTokenStream},
};

use crate::context::ResolvedContext;

pub struct CliMigrateCodeGen;

impl CodeGen<ResolvedContext> for CliMigrateCodeGen {
    fn generate_contribution(
        &self,
        _ctx: &GenerationContext<ResolvedContext>,
        point: &str,
    ) -> Result<ErasedContributionValue, GeneratorError> {
        match point {
            "cli::mod::use" => Ok(ErasedContributionValue::new(RenderedTokenStream::from(
                quote! {
                    mod migrate;
                },
            ))),
            "cli::mod::variants" => Ok(ErasedContributionValue::new(RenderedTokenStream::from(
                quote! {
                    /// Apply pending database migrations and exit.
                    Migrate,
                },
            ))),
            "cli::mod::arms" => Ok(ErasedContributionValue::new(RenderedTokenStream::from(
                quote! {
                    Command::Migrate => migrate::migrate().await,
                },
            ))),
            _ => Err(GeneratorError::unexpected(format!("Unknown extension point '{}'", point))),
        }
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn context() -> GenerationContext<ResolvedContext> {
        GenerationContext::new(
            serde_json::from_value(json!({
                "slug": "assistant",
                "agent_name": "assistant",
                "runtime": { "default_tenant_id": "default" },
                "providers": [],
                "agent": {
                    "version": "0.1.0",
                    "description": null,
                    "prompt": null,
                    "capabilities": null,
                    "capability_policy": null,
                    "model": { "provider": "anthropic", "name": "claude" }
                },
                "blocks": {},
                "tools": {},
                "skills": {},
                "http_server": null
            }))
            .unwrap(),
        )
    }

    #[test]
    fn migrate_command_contributes_cli_dispatch() {
        assert!(
            CliMigrateCodeGen
                .generate_contribution(&context(), "cli::mod::use")
                .unwrap()
                .downcast::<RenderedTokenStream>()
                .unwrap()
                .as_str()
                .contains("mod migrate ;")
        );
        assert!(
            CliMigrateCodeGen
                .generate_contribution(&context(), "cli::mod::variants")
                .unwrap()
                .downcast::<RenderedTokenStream>()
                .unwrap()
                .as_str()
                .contains("Migrate ,")
        );
        assert!(
            CliMigrateCodeGen
                .generate_contribution(&context(), "cli::mod::arms")
                .unwrap()
                .downcast::<RenderedTokenStream>()
                .unwrap()
                .as_str()
                .contains("Command :: Migrate => migrate :: migrate ()")
        );
    }
}
