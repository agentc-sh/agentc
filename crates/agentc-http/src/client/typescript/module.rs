// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::{borrow::Cow, sync::OnceLock};

use agentc_executor_typescript::guestjs::{
    errors::Error,
    host::{Exports, HostClass, HostModule},
    marshal::ToGuestBound,
    runtime::Scope,
};

use crate::client::{
    builder::HttpClientBuilder,
    client::HttpClient,
    errors::HttpClientError,
    typescript::{fetch::FetchRequest, headers::Headers, response::Response},
};

/// A GuestJS host module exposing an [`HttpClient`](crate::client::client::HttpClient) as the Web
/// Fetch API.
pub struct HttpModule {
    specifier: Cow<'static, str>,
    source: ClientSource,
}

impl HttpModule {
    const DEFAULT_SPECIFIER: &'static str = "agentc:http";

    fn client(&self) -> Result<HttpClient, HttpClientError> {
        match &self.source {
            ClientSource::Ready(client) => Ok(client.clone()),
            ClientSource::PerGuest { builder, client } => match client.get() {
                Some(client) => Ok(client.clone()),
                None => Ok(client
                    .get_or_init(|| {
                        builder
                            .build()
                            .expect("client configuration is valid")
                    })
                    .clone()),
            },
        }
    }

    /// Binds an existing client.
    pub fn new(client: impl Into<HttpClient>) -> Self {
        Self {
            specifier: Cow::Borrowed(Self::DEFAULT_SPECIFIER),
            source: ClientSource::Ready(client.into()),
        }
    }

    /// Builds one client per bound guest from a shared configuration.
    ///
    /// A `reqwest` connection pool belongs to the Tokio runtime that created it, and each executor
    /// worker runs its own runtime, so a client built once and shared across workers can outlive
    /// the runtime driving its connections.
    pub fn per_guest(builder: impl Into<HttpClientBuilder>) -> Self {
        Self {
            specifier: Cow::Borrowed(Self::DEFAULT_SPECIFIER),
            source: ClientSource::PerGuest {
                builder: builder.into(),
                client: OnceLock::new(),
            },
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
}

impl Clone for HttpModule {
    /// Clones the configuration without the built client, so each guest builds its own.
    fn clone(&self) -> Self {
        Self {
            specifier: self.specifier.clone(),
            source: match &self.source {
                ClientSource::Ready(client) => ClientSource::Ready(client.clone()),
                ClientSource::PerGuest { builder, .. } => ClientSource::PerGuest {
                    builder: builder.clone(),
                    client: OnceLock::new(),
                },
            },
        }
    }
}

impl HostModule for HttpModule {
    fn name(&self) -> &str {
        self.specifier()
    }

    fn initialize<'js>(&self, scope: &Scope<'js>) -> Result<(), Error> {
        let module = scope.host_module(self.specifier())?;
        let globals = scope.ctx().globals();

        globals.set(
            "fetch",
            module
                .function("fetch")?
                .to_guest_bound(scope)?,
        )?;
        globals.set(
            Headers::NAME,
            module
                .class(Headers::NAME)?
                .to_guest_bound(scope)?,
        )?;
        globals.set(
            Response::NAME,
            module
                .class(Response::NAME)?
                .to_guest_bound(scope)?,
        )?;

        Ok(())
    }

    fn build(&self, exports: &mut Exports) {
        let client = self.client();

        exports.class::<Headers>();
        exports.class::<Response>();

        exports.async_function("fetch", move |scope, args| {
            let request = FetchRequest::from_args(scope, &args)?;
            let client = client
                .as_ref()
                .map_err(|error| Error::unexpected(format!("agentc:http: {error}")))?
                .clone();

            Ok(async move { Response::send(&client, request).await })
        });
    }
}

enum ClientSource {
    Ready(HttpClient),
    PerGuest {
        builder: HttpClientBuilder,
        client: OnceLock<HttpClient>,
    },
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use agentc_executor_typescript::{executor::Executor, guestjs::handle::Promise};
    use axum::{Router, routing::get};
    use tokio::net::TcpListener;

    use super::*;
    use crate::client::policies::address::PublicAddressPolicy;

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

    async fn executor(builder: HttpClientBuilder) -> Executor {
        Executor::builder("fetch.ts", FETCH_SOURCE)
            .workers(1)
            .standard_environment()
            .configure(move |guest| guest.bind(HttpModule::per_guest(builder.clone())))
            .build()
            .await
            .expect("executor builds")
    }

    async fn call(executor: &Executor, export: &'static str, url: String) -> String {
        executor
            .execute(move |context| {
                Box::pin(async move {
                    context
                        .module()
                        .function(export)
                        .await?
                        .call::<_, Promise<String>>((url,))
                        .await?
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
                                .function("stream")
                                .await?
                                .call::<_, Promise<i32>>((url,))
                                .await?
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
        let executor = Executor::builder("globals.ts", GLOBALS_SOURCE)
            .workers(1)
            .standard_environment()
            .configure(|guest| guest.bind(HttpModule::per_guest(HttpClient::builder())))
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
                            .function("identical")
                            .await?
                            .call::<_, Promise<String>>(())
                            .await?
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
