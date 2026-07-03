// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

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
