// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::cell::RefCell;

use agentc_executor_python::{
    backend::ExecutorBackend,
    guestpy::{
        FromGuest,
        errors::Error,
        handle::{AsyncIter, AsyncIterable, Instance},
        host_class,
    },
};
use bytes::Bytes;

use crate::client::python::params::QueryParams;

pub struct Json {
    pub(crate) value: serde_json::Value,
}

#[host_class(backend = B, crate_path = agentc_executor_python::guestpy)]
impl Json {
    #[guestpy(constructor)]
    fn new(value: agentc_executor_python::json::Json) -> Result<Self, Error> {
        Ok(Self { value: value.into_inner() })
    }
}

pub struct Form {
    pub(crate) fields: Vec<(String, String)>,
}

#[host_class(backend = B, crate_path = agentc_executor_python::guestpy)]
impl Form {
    #[guestpy(constructor)]
    fn new(fields: QueryParams) -> Result<Self, Error> {
        Ok(Self { fields: fields.pairs() })
    }
}

#[derive(FromGuest)]
#[guestpy(union, backend = B, crate_path = agentc_executor_python::guestpy)]
pub enum Body<B: ExecutorBackend> {
    Bytes(Bytes),
    Text(String),
    Json(Instance<B, Json>),
    Form(Instance<B, Form>),
    Stream(AsyncIterable<B, Bytes>),
}

pub enum RequestBody<B: ExecutorBackend> {
    Bytes(Bytes),
    Text(String),
    Json(serde_json::Value),
    Form(Vec<(String, String)>),
    Stream(RefCell<Option<AsyncIter<B, Bytes>>>),
}

impl<B: ExecutorBackend> RequestBody<B> {
    pub(crate) fn from_body(body: Body<B>) -> Result<Self, Error> {
        Ok(match body {
            Body::Bytes(bytes) => Self::Bytes(bytes),
            Body::Text(text) => Self::Text(text),
            Body::Json(json) => Self::Json(json.borrow_with(|json| json.value.clone())?),
            Body::Form(form) => Self::Form(form.borrow_with(|form| form.fields.clone())?),
            Body::Stream(stream) => Self::Stream(RefCell::new(Some(stream.into_inner()))),
        })
    }

    pub(crate) fn content_type(&self) -> Option<&'static str> {
        match self {
            Self::Bytes(_) | Self::Stream(_) => None,
            Self::Text(_) => Some("text/plain; charset=utf-8"),
            Self::Json(_) => Some("application/json"),
            Self::Form(_) => Some("application/x-www-form-urlencoded"),
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
            handle::ObjectProtocol,
            host::{library::HostLibrary, module::ModuleSpec},
            marshal::FromGuest,
            rustpython::RustPython,
        },
    };

    use super::{Form, Json};

    const SOURCE: &str = r#"
from agentc_http import Form, Json


def inspect_bodies():
    invalid_json = ""

    Json({"a": [1, True, None]})
    Form({"a": ["x", "y"], "b": True})

    try:
        Json(object())
    except TypeError:
        invalid_json = "TypeError"

    return invalid_json
"#;

    fn module() -> Result<ModuleSpec<RustPython>, Error> {
        ModuleSpec::new("agentc_http")
            .class::<Json>()?
            .class::<Form>()
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
    async fn json_and_form_convert() {
        let executor = executor().await;

        assert_eq!(call::<String>(&executor, "inspect_bodies").await, "TypeError",);

        executor
            .shutdown()
            .await
            .expect("executor shuts down");
    }
}
