// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::time::Duration;

use agentc_executor_python::{
    backend::ExecutorBackend,
    guestpy::{
        errors::Error,
        host::exception::{ExceptionClass, Raise},
        host_class,
    },
};
use http::{HeaderMap, Method};

use crate::client::python::{
    body::{Body, RequestBody},
    headers::{HeaderTypes, Headers},
    params::QueryParams,
    timeout::FromSeconds,
};

pub struct Request<B: ExecutorBackend> {
    pub(crate) method: Method,
    pub(crate) url: String,
    pub(crate) params: Vec<(String, String)>,
    pub(crate) headers: HeaderMap,
    pub(crate) body: Option<RequestBody<B>>,
    pub(crate) timeout: Option<Duration>,
}

#[host_class(backend = B, crate_path = agentc_executor_python::guestpy)]
impl<B: ExecutorBackend> Request<B> {
    #[guestpy(constructor)]
    fn new(
        method: String,
        url: String,
        #[guestpy(kw)] params: Option<QueryParams>,
        #[guestpy(kw)] headers: Option<HeaderTypes<B>>,
        #[guestpy(kw)] body: Option<Body<B>>,
        #[guestpy(kw)] timeout: Option<f64>,
    ) -> Result<Self, Error> {
        Ok(Self {
            method: Method::from_bytes(method.to_uppercase().as_bytes()).map_err(|_| {
                Raise::<B>::new(ExceptionClass::builtin("ValueError"))
                    .arg(format!("invalid HTTP method: {method}"))
            })?,
            url,
            params: params
                .map(QueryParams::pairs)
                .unwrap_or_default(),
            headers: match headers {
                Some(headers) => Headers::map(headers)?,
                None => HeaderMap::new(),
            },
            body: body
                .map(RequestBody::from_body)
                .transpose()?,
            timeout: timeout
                .map(Duration::from_seconds::<B>)
                .transpose()?,
        })
    }

    #[guestpy(get)]
    fn method(&self) -> Result<String, Error> {
        Ok(self.method.as_str().to_owned())
    }

    #[guestpy(get)]
    fn url(&self) -> Result<String, Error> {
        Ok(self.url.clone())
    }

    #[guestpy(get)]
    fn headers(&self) -> Result<Headers<B>, Error> {
        Ok(Headers::from_map(self.headers.clone()))
    }
}

#[cfg(test)]
mod tests {
    use agentc_executor_python::{
        executor::Executor,
        guestpy::{
            bundle::Bundle,
            errors::Error,
            handle::ObjectProtocol,
            host::{library::HostLibrary, module::ModuleSpec},
            marshal::FromGuest,
            rustpython::RustPython,
        },
    };

    use super::Request;
    use crate::client::python::{
        body::{Form, Json},
        headers::Headers,
    };

    const SOURCE: &str = r#"
from agentc_http import Json, Request


def inspect_requests():
    request = Request("get", "/users", headers={"Accept": "text/plain"})

    try:
        Request("in valid", "/")
    except ValueError as error:
        bad_method = error.args[0]

    try:
        Request("GET", "/", timeout=-1.0)
    except ValueError as error:
        bad_timeout = error.args[0]

    try:
        Request("GET", "/", body=object())
    except TypeError:
        bad_body = "TypeError"

    return [
        request.method,
        request.url,
        request.headers["accept"],
        request.headers == request.headers,
        "content-type" in Request("POST", "/", body=Json({"a": 1})).headers,
        bad_method,
        bad_timeout,
        bad_body,
    ]
"#;

    fn module() -> Result<ModuleSpec<RustPython>, Error> {
        ModuleSpec::new("agentc_http")
            .class::<Headers<RustPython>>()?
            .class::<Json>()?
            .class::<Form>()?
            .class::<Request<RustPython>>()
    }

    async fn executor() -> Executor<RustPython> {
        Executor::<RustPython>::builder("agentc_http_unit_test")
            .bundle(Bundle::single("agentc_http_unit_test", SOURCE).expect("the bundle builds"))
            .workers(1)
            .configure(|runtime| Ok(runtime.bind(HostLibrary::new().with(module()?))))
            .build()
            .await
            .expect("executor builds")
    }

    async fn call<T>(executor: &Executor<RustPython>, export: &'static str) -> T::Owned
    where
        T: FromGuest<RustPython>,
        T::Owned: Send + 'static,
    {
        executor
            .execute(move |context| {
                Box::pin(async move {
                    context
                        .module()
                        .function(export)?
                        .call::<_, T>(())
                })
            })
            .await
            .expect("guest call succeeds")
    }

    #[tokio::test]
    async fn requests_validate_construction() {
        let executor = executor().await;

        assert_eq!(
            call::<agentc_executor_python::json::Json>(&executor, "inspect_requests")
                .await
                .into_inner(),
            serde_json::json!([
                "GET",
                "/users",
                "text/plain",
                true,
                false,
                "invalid HTTP method: in valid",
                "invalid timeout: -1",
                "TypeError",
            ]),
        );

        executor
            .shutdown()
            .await
            .expect("executor shuts down");
    }
}
