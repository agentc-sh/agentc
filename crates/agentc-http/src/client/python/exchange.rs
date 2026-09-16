// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use agentc_executor_python::{
    backend::ExecutorBackend,
    guestpy::{handle::Instance, host::exception::Raise},
};

use crate::client::{
    errors::HttpClientError,
    python::{
        exceptions::{
            DecodingError, InvalidRequest, RequestDenied, ResponseTooLarge, TimeoutException,
            TooManyRedirects, TransportError,
        },
        request::Request,
        response::Response,
    },
};

pub(crate) enum Exchange<B: ExecutorBackend> {
    Sending {
        request: Instance<B, Request<B>>,
    },
    Streaming {
        request: Instance<B, Request<B>>,
        response: Instance<B, Response<B>>,
    },
}

impl<B: ExecutorBackend> Exchange<B> {
    pub(crate) fn raise(&self, error: HttpClientError) -> Raise<B> {
        let message = error.to_string();

        match (self, error) {
            (Self::Sending { request }, HttpClientError::Denied { policy, reason })
            | (Self::Streaming { request, .. }, HttpClientError::Denied { policy, reason }) => {
                Raise::host(RequestDenied {
                    message,
                    request: request.clone(),
                    policy: policy.to_owned(),
                    reason,
                })
            }
            (Self::Sending { request }, HttpClientError::Timeout)
            | (Self::Streaming { request, .. }, HttpClientError::Timeout) => {
                Raise::host(TimeoutException { message, request: request.clone() })
            }
            (Self::Streaming { response, .. }, HttpClientError::BodyTooLarge { limit }) => {
                Raise::host(ResponseTooLarge {
                    message,
                    response: response.clone(),
                    limit,
                })
            }
            (Self::Sending { request }, HttpClientError::TooManyRedirects) => {
                Raise::host(TooManyRedirects { message, request: request.clone() })
            }
            (Self::Sending { request }, HttpClientError::InvalidRequest { .. }) => {
                Raise::host(InvalidRequest { message, request: request.clone() })
            }
            (Self::Sending { request }, HttpClientError::Configuration { .. })
            | (Self::Sending { request }, HttpClientError::Transport { .. })
            | (Self::Streaming { request, .. }, HttpClientError::Transport { .. }) => {
                Raise::host(TransportError { message, request: request.clone() })
            }
            (Self::Streaming { response, .. }, HttpClientError::Decode { .. }) => {
                Raise::host(DecodingError { message, response: response.clone() })
            }
            (Self::Sending { .. }, HttpClientError::BodyTooLarge { .. })
            | (Self::Sending { .. }, HttpClientError::Decode { .. })
            | (Self::Streaming { .. }, HttpClientError::TooManyRedirects)
            | (Self::Streaming { .. }, HttpClientError::InvalidRequest { .. })
            | (Self::Streaming { .. }, HttpClientError::Configuration { .. }) => {
                unreachable!("HTTP client error does not belong to this exchange phase")
            }
        }
    }
}

impl<B: ExecutorBackend> Clone for Exchange<B> {
    fn clone(&self) -> Self {
        match self {
            Self::Sending { request } => Self::Sending { request: request.clone() },
            Self::Streaming { request, response } => Self::Streaming {
                request: request.clone(),
                response: response.clone(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use agentc_executor_python::{
        executor::Executor,
        guestpy::{
            bundle::Bundle,
            errors::Error,
            handle::{Class, Instance, ObjectProtocol},
            host::{library::HostLibrary, module::ModuleSpec},
            marshal::FromGuest,
            rustpython::RustPython,
        },
    };

    use super::*;
    use crate::client::{
        client::HttpClient,
        python::{exceptions::BodyClosed, module::HttpModule},
        response::HttpResponse,
    };

    const SOURCE: &str = r#"
import agentc_http


def classify(kind):
    try:
        agentc_http.fail(kind)
    except BaseException as error:
        return type(error).__name__

    return "no exception"


def bases(kind):
    try:
        agentc_http.fail(kind)
    except agentc_http.HTTPError:
        return "HTTPError"
    except RuntimeError:
        return "RuntimeError"
    except BaseException:
        return "neither"

    return "no exception"


def carried(kind):
    try:
        agentc_http.fail(kind)
    except agentc_http.RequestDenied as error:
        return f"{error.policy}:{error.reason}"
    except agentc_http.ResponseTooLarge as error:
        return str(error.limit)

    return "no exception"


def message(kind):
    try:
        agentc_http.fail(kind)
    except BaseException as error:
        return str(error)

    return "no exception"


def identity(kind):
    request = agentc_http.Request("GET", "https://example.test/")

    try:
        agentc_http.fail(kind, request)
    except agentc_http.ResponseError as error:
        return error.response.request is request
    except agentc_http.RequestError as error:
        return error.request is request

    return False


def closed_is_runtime_error():
    return issubclass(agentc_http.BodyClosed, RuntimeError)
"#;

    fn module() -> Result<ModuleSpec<RustPython>, Error> {
        let module: ModuleSpec<RustPython> = HttpModule::new(
            HttpClient::builder()
                .build()
                .expect("client builds"),
        )
        .try_into()?;

        Ok(
            module.function("fail", |enter, args| {
                let kind = args.required::<String>(enter, 0, "kind")?;
                let request = args.optional::<Instance<RustPython, Request<RustPython>>>(
                    enter,
                    1,
                    "request",
                )?;

                args.finish()?;

                if kind == "closed" {
                    return Err(
                        Raise::<RustPython>::host(
                            BodyClosed {
                                message: String::from("body closed before consuming"),
                            },
                        )
                        .into(),
                    );
                }

                let request = match request {
                    Some(request) => request,
                    None => Class::of::<Request<RustPython>>(enter)?.construct((
                        String::from("GET"),
                        String::from("https://example.test/"),
                    ))?,
                };

                let response = Class::of::<Response<RustPython>>(enter)?.instantiate(
                    Response::from_response(
                        HttpResponse::new(
                            reqwest::Response::from(
                                http::Response::builder()
                                    .status(200)
                                    .body(reqwest::Body::from("payload"))
                                    .expect("test response builds"),
                            ),
                            None,
                            None,
                        ),
                        request.clone(),
                    )
                )?;

                let (exchange, error) = match kind.as_str() {
                    "denied" => (
                        Exchange::Sending { request },
                        HttpClientError::Denied {
                            policy: "url-pattern",
                            reason: String::from("nothing is permitted"),
                        },
                    ),
                    "timeout" => (
                        Exchange::Sending { request },
                        HttpClientError::Timeout,
                    ),
                    "too_large" => (
                        Exchange::Streaming {
                            request,
                            response: response.clone(),
                        },
                        HttpClientError::BodyTooLarge { limit: 8 },
                    ),
                    "redirects" => (
                        Exchange::Sending { request },
                        HttpClientError::TooManyRedirects,
                    ),
                    "invalid" => (
                        Exchange::Sending { request },
                        HttpClientError::invalid_request("bad URL"),
                    ),
                    "configuration" => (
                        Exchange::Sending { request },
                        HttpClientError::configuration("bad configuration"),
                    ),
                    "transport" => (
                        Exchange::Sending { request },
                        HttpClientError::Transport {
                            source: reqwest_middleware::Error::Middleware(
                                anyhow::anyhow!("offline"),
                            ),
                        },
                    ),
                    "decode" => (
                        Exchange::Streaming { request, response },
                        HttpClientError::decode("invalid JSON"),
                    ),
                    _ => panic!("unknown test case: {kind}"),
                };

                Err::<(), Error>(
                    exchange.raise(error).into(),
                )
            }),
        )
    }

    async fn executor() -> Executor<RustPython> {
        Executor::<RustPython>::builder("agentc_http_exchange_test")
            .bundle(
                Bundle::single("agentc_http_exchange_test", SOURCE).expect("bundle builds"),
            )
            .workers(1)
            .configure(
                |runtime| Ok(runtime.bind(HostLibrary::new().with(module()?))),
            )
            .build()
            .await
            .expect("executor builds")
    }

    async fn call<T>(
        executor: &Executor<RustPython>,
        export: &'static str,
        kind: &'static str,
    ) -> T::Owned
    where
        T: FromGuest<RustPython>,
        T::Owned: Send + 'static,
    {
        executor
            .execute(
                move |context| Box::pin(
                    async move {
                        context
                            .module()
                            .function(export)?
                            .call::<_, T>((kind,))
                    },
                ),
            )
            .await
            .expect("guest call succeeds")
    }

    #[tokio::test]
    async fn every_host_error_maps_to_its_guest_class() {
        let executor = executor().await;

        for (kind, class) in [
            ("denied", "RequestDenied"),
            ("timeout", "TimeoutException"),
            ("too_large", "ResponseTooLarge"),
            ("redirects", "TooManyRedirects"),
            ("invalid", "InvalidRequest"),
            ("configuration", "TransportError"),
            ("transport", "TransportError"),
            ("decode", "DecodingError"),
        ] {
            assert_eq!(
                call::<String>(&executor, "classify", kind).await,
                class,
            );
            assert!(
                call::<bool>(&executor, "identity", kind).await,
            );
        }

        assert_eq!(
            call::<String>(&executor, "bases", "timeout").await,
            "HTTPError",
        );
        assert_eq!(
            call::<String>(&executor, "bases", "closed").await,
            "RuntimeError",
        );
        assert_eq!(
            call::<String>(&executor, "carried", "denied").await,
            "url-pattern:nothing is permitted",
        );
        assert_eq!(
            call::<String>(&executor, "carried", "too_large").await,
            "8",
        );
        assert_eq!(
            call::<String>(&executor, "message", "denied").await,
            "denied by policy 'url-pattern': nothing is permitted",
        );
        assert_eq!(
            call::<String>(&executor, "message", "too_large").await,
            "response body exceeded the limit of 8 bytes",
        );

        for (kind, message) in [
            ("timeout", "request timed out"),
            ("redirects", "too many redirects"),
            ("invalid", "invalid request: bad URL"),
            ("configuration", "invalid configuration: bad configuration"),
            ("transport", "transport error: offline"),
            ("decode", "failed to decode the response: invalid JSON"),
        ] {
            assert_eq!(
                call::<String>(&executor, "message", kind).await,
                message,
            );
        }

        assert!(
            executor
                .execute(
                    |context| Box::pin(
                        async move {
                            context
                                .module()
                                .function("closed_is_runtime_error")?
                                .call::<_, bool>(())
                        },
                    ),
                )
                .await
                .expect("guest call succeeds"),
        );

        executor.shutdown().await.expect("executor shuts down");
    }
}
