// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use agentc_executor_typescript::{
    backend::ExecutorBackend, errors::Error, executor::ExecutorBuilder,
};

use crate::client::{builder::HttpClientBuilder, typescript::library::HttpLibrary};

/// Extends a TypeScript executor builder with the `agentc:http` host module.
pub trait ExecutorBuilderHttpExt {
    /// Binds `agentc:http` on every worker, built from one shared client configuration.
    fn with_http(self, builder: impl Into<HttpClientBuilder>) -> Self;
}

impl<B: ExecutorBackend> ExecutorBuilderHttpExt for ExecutorBuilder<B> {
    fn with_http(self, builder: impl Into<HttpClientBuilder>) -> Self {
        let builder = builder.into();

        self.configure(move |runtime| {
            Ok(runtime.bind(HttpLibrary::bind::<B>(
                builder
                    .clone()
                    .build()
                    .map_err(Error::configuration)?,
            )?))
        })
    }
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use agentc_executor_typescript::{
        executor::Executor,
        guestjs::{
            bundle::Bundle,
            handle::{
                promise::Promise,
                protocol::{CallableProtocol, ObjectProtocol},
            },
            quickjs::QuickJs,
        },
    };
    use axum::{Router, routing::get};
    use tokio::net::TcpListener;

    use super::*;
    use crate::client::client::HttpClient;

    const FETCH_SOURCE: &str = r#"
import { fetch } from "agentc:http";

export async function read(url) {
    const response = await fetch(url);

    return `${response.status}:${await response.text()}`;
}
"#;

    async fn server() -> SocketAddr {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("test listener binds");
        let address = listener
            .local_addr()
            .expect("test listener reports its address");

        drop(tokio::spawn(async move {
            axum::serve(listener, Router::new().route("/ok", get(|| async { "hello" }))).await
        }));

        address
    }

    #[tokio::test]
    async fn the_extension_trait_binds_the_module_on_every_worker() {
        let address = server().await;
        let executor = Executor::<QuickJs>::builder("fetch")
            .bundle(Bundle::single("fetch.ts", FETCH_SOURCE).unwrap())
            .workers(2)
            .standard_environment()
            .with_http(HttpClient::builder())
            .build()
            .await
            .expect("executor builds");

        for _ in 0..4 {
            assert_eq!(
                executor
                    .execute({
                        let url = format!("http://{address}/ok");

                        move |context| {
                            Box::pin(async move {
                                context
                                    .module()
                                    .function("read")?
                                    .call::<_, Promise<QuickJs, String>>((url,))?
                                    .await
                            })
                        }
                    })
                    .await
                    .expect("guest call succeeds"),
                "200:hello",
            );
        }

        executor
            .shutdown()
            .await
            .expect("executor shuts down");
    }

    #[tokio::test]
    async fn an_invalid_client_configuration_fails_the_build() {
        let Err(error) = Executor::<QuickJs>::builder("fetch")
            .bundle(Bundle::single("fetch.ts", FETCH_SOURCE).unwrap())
            .workers(1)
            .standard_environment()
            .with_http(HttpClient::builder().header("in valid", "x"))
            .build()
            .await
        else {
            panic!("an invalid client configuration should fail the build");
        };

        let Error::WorkerInitialization { source, .. } = error else {
            panic!("invalid HTTP configuration should fail worker initialization");
        };

        assert!(matches!(*source, Error::Configuration(_)),);
    }
}
