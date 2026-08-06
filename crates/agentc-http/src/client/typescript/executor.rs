// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use agentc_executor_typescript::executor::ExecutorBuilder;

use crate::client::{builder::HttpClientBuilder, typescript::library::HttpLibrary};

/// Extends a TypeScript executor builder with the `agentc:http` host module.
pub trait ExecutorBuilderHttpExt {
    /// Binds `agentc:http` on every worker, built from one shared client configuration.
    fn with_http(self, builder: impl Into<HttpClientBuilder>) -> Self;
}

impl ExecutorBuilderHttpExt for ExecutorBuilder {
    fn with_http(self, builder: impl Into<HttpClientBuilder>) -> Self {
        let builder = builder.into();

        self.configure(move |runtime| runtime.bind(HttpLibrary::bind_guest(builder.clone())))
    }
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use agentc_executor_typescript::{executor::Executor, guestjs::handle::Promise};
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
        let executor = Executor::builder("fetch.ts", FETCH_SOURCE)
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
                                    .function("read")
                                    .await?
                                    .call::<_, Promise<String>>((url,))
                                    .await?
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
}
