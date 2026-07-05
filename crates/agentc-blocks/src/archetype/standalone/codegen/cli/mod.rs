// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

pub mod config;
pub mod run;
pub mod serve;
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
            use tokio::signal;
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
                /// Print the resolved runtime configuration as JSON.
                Config,

                #extra_variants
            }

            pub async fn run(telemetry: Telemetry) -> Result<()> {
                let cli = Cli::parse();

                match cli.command {
                    Command::Run(args) => {
                        telemetry.disable_logging();
                        run::run(args).await
                    },
                    Command::Config => config::config().await,

                    #extra_arms
                }
            }
        };

        Ok(vec![("src/cli/mod.rs".into(), source)])
    }
}
