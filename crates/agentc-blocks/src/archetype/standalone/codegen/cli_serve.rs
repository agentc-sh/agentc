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

pub struct CliServeCodeGen;

impl CodeGen<ResolvedContext> for CliServeCodeGen {
    fn generate_contribution(
        &self,
        _ctx: &GenerationContext<ResolvedContext>,
        point: &str,
    ) -> Result<TokenStream, GeneratorError> {
        match point {
            "cli::mod::use" => Ok(quote! {
                mod serve;
            }),
            "cli::mod::variants" => Ok(quote! {
                /// Start the HTTP server.
                Serve(serve::ServeArgs),
            }),
            "cli::mod::arms" => Ok(quote! {
                Command::Serve(args) => serve::run(args).await,
            }),
            _ => Err(GeneratorError::unexpected(format!("Unknown extension point '{}'", point))),
        }
    }

    fn generate_files(
        &self,
        _ctx: &GenerationContext<ResolvedContext>,
        _registry: &ExtensionRegistry,
    ) -> Result<Vec<(PathBuf, TokenStream)>, GeneratorError> {
        let source = quote! {
            use std::{sync::Arc, time::Duration};
            use anyhow::Result;
            use clap::Args;
            use tokio_util::sync::CancellationToken;

            use agentc_telemetry::info;
            use agentc_agent_react::service::ApplicationService;

            use crate::cli::shutdown::ShutdownSignal;
            use crate::config::Config;
            use crate::agent::build_agent;
            use crate::server;

            #[derive(Args, Debug)]
            pub struct ServeArgs {}

            pub async fn run(_args: ServeArgs) -> Result<()> {
                let shutdown = CancellationToken::new();

                info!(
                    event = "StartingServer",
                    version = env!("CARGO_PKG_VERSION")
                );

                let config = Config::load().await?;

                let database = Arc::new(config.database.build(true).await?);

                info!(event = "DatabaseInitialized");

                let agent = build_agent(database.clone(), &config, shutdown.clone()).await?;

                info!(
                    event = "AgentInitialized",
                    name = agent.identity().name,
                    provider = agent.identity().provider,
                    model = agent.identity().model,
                    capabilities = ?agent.identity().capabilities.iter().map(|c| c.as_str()).collect::<Vec<_>>(),
                );

                let service = Arc::new(
                    ApplicationService::builder()
                        .with_agent(agent)
                        .with_database(database)
                        .build()
                );

                let mut server = server::build(service, &config)?;
                server.spawn();

                info!(
                    event = "Listening",
                    address = ?server.address()
                );

                shutdown.shutdown_signal().await;

                info!(
                    event = "ShuttingDown",
                );

                server.graceful_shutdown(Some(Duration::from_secs(30)));
                tokio::time::timeout(Duration::from_secs(35), server.join())
                    .await
                    .ok();

                Ok(())
            }
        };

        Ok(vec![("src/cli/serve.rs".into(), source)])
    }
}
