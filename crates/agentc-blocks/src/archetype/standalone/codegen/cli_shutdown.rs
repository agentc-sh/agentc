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

pub struct CliShutdownCodeGen;

impl CodeGen<ResolvedContext> for CliShutdownCodeGen {
    fn generate_files(
        &self,
        _ctx: &GenerationContext<ResolvedContext>,
        _registry: &ExtensionRegistry,
    ) -> Result<Vec<(PathBuf, TokenStream)>, GeneratorError> {
        let source = quote! {
            use async_trait::async_trait;
            use tokio::signal;
            use tokio_util::sync::CancellationToken;

            /// Resolution of process shutdown: token cancellation or an OS signal,
            /// whichever lands first.
            #[async_trait]
            pub trait ShutdownSignal {
                async fn shutdown_signal(&self);
            }

            #[async_trait]
            impl ShutdownSignal for CancellationToken {
                async fn shutdown_signal(&self) {
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
                        _ = self.cancelled() => (),
                        _ = ctrl_c => (),
                        _ = terminate => (),
                        _ = interrupt => (),
                        _ = hangup => (),
                    }
                }
            }
        };

        Ok(vec![("src/cli/shutdown.rs".into(), source)])
    }
}
