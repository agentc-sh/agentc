// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

pub mod config;
pub mod shutdown;

use proc_macro2::TokenStream;
use quote::quote;
use std::path::PathBuf;

use agentc_compiler::generator::{
    blocks::codegen::CodeGen, context::GenerationContext, errors::GeneratorError,
    extension::ExtensionRegistry,
};

use crate::context::ResolvedContext;

pub struct CliModCodeGen;

impl CodeGen<ResolvedContext> for CliModCodeGen {
    fn generate_files(
        &self,
        _ctx: &GenerationContext<ResolvedContext>,
        registry: &ExtensionRegistry,
    ) -> Result<Vec<(PathBuf, TokenStream)>, GeneratorError> {
        let extra_use = registry
            .get("cli::mod::use")
            .and_then(|s| s.parse::<TokenStream>().ok());

        let extra_variants = registry
            .get("cli::mod::variants")
            .and_then(|s| s.parse::<TokenStream>().ok());

        let extra_arms = registry
            .get("cli::mod::arms")
            .and_then(|s| s.parse::<TokenStream>().ok());

        let source = quote! {
            use anyhow::Result;
            use clap::{Parser, Subcommand};
            use agentc_telemetry::Telemetry;

            mod shutdown;
            mod run;
            mod config;

            #extra_use

            #[derive(Parser, Debug)]
            #[command(author, version, about, long_about = None)]
            struct Cli {
                #[command(subcommand)]
                command: Command,
            }

            #[derive(Subcommand, Debug)]
            enum Command {
                /// Run the agent with the given input.
                Run(run::RunArgs),
                /// Print the resolved runtime configuration.
                ///
                /// Defaults to a human-readable format with secrets redacted;
                /// pass `--format json` for the resolved JSON with real values.
                Config(config::ConfigArgs),

                #extra_variants
            }

            pub async fn run(telemetry: Telemetry) -> Result<()> {
                let cli = Cli::parse();

                match cli.command {
                    Command::Run(args) => {
                        telemetry.disable_logging();
                        run::run(args).await
                    },
                    Command::Config(args) => config::config(args).await,

                    #extra_arms
                }
            }
        };

        Ok(vec![("src/cli/mod.rs".into(), source)])
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
    fn config_variant_carries_config_args() {
        let source = CliModCodeGen
            .generate_files(&context(), &ExtensionRegistry::empty())
            .unwrap()[0]
            .1
            .to_string();

        assert!(source.contains("Config (config :: ConfigArgs)"));
        assert!(source.contains("Command :: Config (args) => config :: config (args)"));
    }
}
