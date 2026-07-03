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
            use tokio_util::sync::CancellationToken;
            use agentc_telemetry::Telemetry;

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

            /// Wait for an OS signal or an external cancellation, then cancel the token
            /// so all components sharing it see the shutdown.
            ///
            /// Any subcommand that needs orderly shutdown can call this with a clone of
            /// its root [`CancellationToken`].
            pub async fn shutdown_signal(shutdown: CancellationToken) {
                let ctrl_c = async {
                    signal::ctrl_c()
                        .await
                        .expect("Failed to install CTRL+C signal handler");
                };

                #[cfg(unix)]
                let terminate = async {
                    signal::unix::signal(signal::unix::SignalKind::terminate())
                        .expect("Failed to install SIGTERM signal handler")
                        .recv()
                        .await;
                };
                #[cfg(unix)]
                let interrupt = async {
                    signal::unix::signal(signal::unix::SignalKind::interrupt())
                        .expect("Failed to install SIGINT signal handler")
                        .recv()
                        .await;
                };
                #[cfg(unix)]
                let hangup = async {
                    signal::unix::signal(signal::unix::SignalKind::hangup())
                        .expect("Failed to install SIGHUP signal handler")
                        .recv()
                        .await;
                };

                #[cfg(not(unix))]
                let terminate = std::future::pending::<()>();
                #[cfg(not(unix))]
                let interrupt = std::future::pending::<()>();
                #[cfg(not(unix))]
                let hangup = std::future::pending::<()>();

                tokio::select! {
                    _ = shutdown.cancelled() => (),
                    _ = ctrl_c => (),
                    _ = terminate => (),
                    _ = interrupt => (),
                    _ = hangup => (),
                }

                shutdown.cancel();
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
