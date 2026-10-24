// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::borrow::Cow;

use agentc_executor_typescript::{
    backend::ExecutorBackend,
    guestjs::{
        errors::Error,
        eval::GuestGlobalsExt,
        handle::{class::Payload, protocol::ObjectProtocol},
        host::{
            library::{HostInitializer, HostLibrary},
            module::ModuleDeclaration,
            state::ModuleState,
        },
        imports::GuestModulesExt,
    },
};

use crate::client::{
    client::HttpClient,
    typescript::{fetch::FetchRequest, headers::Headers, response::Response},
};

/// A GuestJS host module exposing an [`HttpClient`](crate::client::client::HttpClient) as the Web
/// Fetch API.
#[derive(Clone)]
pub struct HttpModule {
    specifier: Cow<'static, str>,
    client: HttpClient,
}

impl HttpModule {
    const DEFAULT_SPECIFIER: &'static str = "agentc:http";

    /// Binds an existing client.
    pub fn new(client: impl Into<HttpClient>) -> Self {
        Self {
            specifier: Cow::Borrowed(Self::DEFAULT_SPECIFIER),
            client: client.into(),
        }
    }

    /// Overrides the import specifier.
    pub fn with_specifier(mut self, specifier: impl Into<Cow<'static, str>>) -> Self {
        self.specifier = specifier.into();
        self
    }

    /// The import specifier.
    pub fn specifier(&self) -> &str {
        &self.specifier
    }

    pub fn client(&self) -> &HttpClient {
        &self.client
    }
}

impl<B: ExecutorBackend> TryFrom<HttpModule> for HostLibrary<B> {
    type Error = Error;

    fn try_from(module: HttpModule) -> Result<Self, Self::Error> {
        let specifier = module.specifier.clone();

        Ok(
            HostLibrary::from(
                ModuleDeclaration::new(specifier.clone())
                    .state(move |_| Ok(module.clone()))
                    .class::<Headers>()
                    .class::<Response<B>>()
                    .function_async("fetch", |context, args| {
                        let client = context
                            .resolve::<ModuleState<HttpModule>>()?
                            .client()
                            .clone();
                        let request = FetchRequest::read(context.enter(), args)?;

                        Ok(async move { Response::<B>::send(&client, request).await })
                    }),
            )
            .extend(HostInitializer::new(move |enter| {
                let module = enter.guest().host_module(&specifier)?;
                let globals = enter.guest().globals()?;

                globals.set("fetch", module.function("fetch")?)?;
                globals.set(Headers::NAME, module.class(Headers::NAME)?)?;
                globals.set(Response::<B>::NAME, module.class(Response::<B>::NAME)?)?;

                Ok(())
            })),
        )
    }
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use agentc_executor_typescript::{
        executor::Executor,
        guestjs::{
            bundle::Bundle,
            handle::{promise::Promise, protocol::CallableProtocol},
            quickjs::QuickJs,
        },
    };
    use axum::{Router, routing::get};
    use tokio::net::TcpListener;

    use super::*;
    use crate::client::{
        builder::HttpClientBuilder,
        policies::address::PublicAddressPolicy,
        typescript::executor::ExecutorBuilderHttpExt,
    };

    const FETCH_SOURCE: &str = r#"
import { fetch } from "agentc:http";

export async function read(url) {
    const response = await fetch(url);

    return `${response.status}:${await response.text()}`;
}

export async function stream(url) {
    const response = await fetch(url);
    const chunks = [];

    for await (const chunk of response.body) {
        chunks.push(...chunk);
    }

    return chunks.length;
}

export async function denied(url) {
    try {
        await fetch(url);

        return "allowed";
    } catch (error) {
        return error.message;
    }
}
"#;

    const GLOBALS_SOURCE: &str = r#"
import {
    fetch as importedFetch,
    Headers as ImportedHeaders,
    Response as ImportedResponse,
} from "agentc:http";

export async function read(url) {
    const response = await fetch(url);

    return `${response.status}:${await response.text()}`;
}

export async function identical() {
    return [
        globalThis.fetch === importedFetch,
        globalThis.Headers === ImportedHeaders,
        globalThis.Response === ImportedResponse,
    ].join(":");
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

    async fn executor(builder: HttpClientBuilder) -> Executor<QuickJs> {
        Executor::<QuickJs>::builder("fetch")
            .bundle(Bundle::single("fetch.ts", FETCH_SOURCE).unwrap())
            .workers(1)
            .standard_environment()
            .with_http(builder)
            .build()
            .await
            .expect("executor builds")
    }

    async fn call(executor: &Executor<QuickJs>, export: &'static str, url: String) -> String {
        executor
            .execute(move |context| {
                Box::pin(async move {
                    context
                        .module()
                        .function(export)?
                        .call::<_, Promise<QuickJs, String>>((url,))?
                        .await
                })
            })
            .await
            .expect("guest call succeeds")
    }

    #[tokio::test]
    async fn guest_reads_a_buffered_body() {
        let address = server().await;
        let executor = executor(HttpClient::builder()).await;

        assert_eq!(call(&executor, "read", format!("http://{address}/ok")).await, "200:hello",);

        executor
            .shutdown()
            .await
            .expect("executor shuts down");
    }

    #[tokio::test]
    async fn guest_reads_a_streamed_body() {
        let address = server().await;
        let executor = executor(HttpClient::builder()).await;

        assert_eq!(
            executor
                .execute({
                    let url = format!("http://{address}/ok");

                    move |context| {
                        Box::pin(async move {
                            context
                                .module()
                                .function("stream")?
                                .call::<_, Promise<QuickJs, i32>>((url,))?
                                .await
                        })
                    }
                })
                .await
                .expect("guest call succeeds"),
            5,
        );

        executor
            .shutdown()
            .await
            .expect("executor shuts down");
    }

    #[tokio::test]
    async fn a_denied_host_surfaces_as_a_guest_exception() {
        let address = server().await;
        let executor = executor(HttpClient::builder().policy(PublicAddressPolicy::default())).await;

        assert!(
            call(&executor, "denied", format!("http://{address}/ok"))
                .await
                .contains("agentc:http:")
        );

        executor
            .shutdown()
            .await
            .expect("executor shuts down");
    }

    #[tokio::test]
    async fn the_exports_are_installed_as_globals() {
        let address = server().await;
        let executor = Executor::<QuickJs>::builder("globals")
            .bundle(Bundle::single("globals.ts", GLOBALS_SOURCE).unwrap())
            .workers(1)
            .standard_environment()
            .with_http(HttpClient::builder())
            .build()
            .await
            .expect("executor builds");

        assert_eq!(call(&executor, "read", format!("http://{address}/ok")).await, "200:hello",);

        assert_eq!(
            executor
                .execute(|context| {
                    Box::pin(async move {
                        context
                            .module()
                            .function("identical")?
                            .call::<_, Promise<QuickJs, String>>(())?
                            .await
                    })
                })
                .await
                .expect("guest call succeeds"),
            "true:true:true",
        );

        executor
            .shutdown()
            .await
            .expect("executor shuts down");
    }
}
