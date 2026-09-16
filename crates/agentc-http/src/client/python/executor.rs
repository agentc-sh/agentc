// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use agentc_executor_python::{backend::ExecutorBackend, errors::Error, executor::ExecutorBuilder};

use crate::client::{builder::HttpClientBuilder, python::library::HttpLibrary};

pub trait ExecutorBuilderHttpExt {
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
    use agentc_executor_python::{
        executor::Executor,
        guestpy::{bundle::Bundle, handle::ObjectProtocol, rustpython::RustPython},
    };

    use super::*;
    use crate::client::client::HttpClient;

    const IMPORT_SOURCE: &str = r#"
import agentc_http


def names():
    return ",".join(
        name
        for name in ("HTTPError", "RequestError", "BodyError")
        if hasattr(agentc_http, name)
    )
"#;

    #[tokio::test]
    async fn the_extension_trait_binds_the_module_on_every_worker() {
        let executor = Executor::<RustPython>::builder("agentc_http_test")
            .bundle(Bundle::single("agentc_http_test", IMPORT_SOURCE).unwrap())
            .workers(2)
            .with_http(HttpClient::builder())
            .build()
            .await
            .expect("executor builds");

        for _ in 0..4 {
            assert_eq!(
                executor
                    .execute(|context| Box::pin(async move {
                        context
                            .module()
                            .function("names")?
                            .call::<_, String>(())
                    },),)
                    .await
                    .expect("guest call succeeds"),
                "HTTPError,RequestError,BodyError",
            );
        }

        executor
            .shutdown()
            .await
            .expect("executor shuts down");
    }

    #[tokio::test]
    async fn an_invalid_client_configuration_fails_the_build() {
        let Err(error) = Executor::<RustPython>::builder("agentc_http_test")
            .bundle(Bundle::single("agentc_http_test", IMPORT_SOURCE).unwrap())
            .workers(1)
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
