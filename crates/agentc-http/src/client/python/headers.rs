// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::marker::PhantomData;

use agentc_executor_python::{
    backend::ExecutorBackend,
    guestpy::{
        FromGuest,
        errors::Error,
        handle::{Instance, Object, ObjectProtocol},
        host::{
            exception::{ExceptionClass, Raise},
            iter::HostIter,
        },
        host_class,
        marshal::collections::{Iterable, Mapping},
    },
};
use http::{HeaderMap, HeaderName, HeaderValue};

#[derive(FromGuest)]
#[guestpy(union, backend = B, crate_path = agentc_executor_python::guestpy)]
pub enum HeaderTypes<B: ExecutorBackend> {
    Headers(Instance<B, Headers<B>>),
    Mapping(Mapping<String, String>),
    Pairs(Iterable<Vec<(String, String)>>),
}

pub struct Headers<B: ExecutorBackend> {
    inner: HeaderMap,
    _marker: PhantomData<fn() -> B>,
}

impl<B: ExecutorBackend> Headers<B> {
    pub(crate) fn from_map(inner: HeaderMap) -> Self {
        Self { inner, _marker: PhantomData }
    }

    pub(crate) fn as_map(&self) -> &HeaderMap {
        &self.inner
    }

    pub(crate) fn map(headers: HeaderTypes<B>) -> Result<HeaderMap, Error> {
        match headers {
            HeaderTypes::Headers(headers) => {
                Ok(headers.borrow_with(|headers| headers.as_map().clone())?)
            }
            HeaderTypes::Mapping(Mapping(pairs)) => Self::collect(pairs),
            HeaderTypes::Pairs(pairs) => Self::collect(pairs.into_inner()),
        }
    }

    fn collect(pairs: Vec<(String, String)>) -> Result<HeaderMap, Error> {
        let mut map = HeaderMap::new();

        for (name, value) in pairs {
            map.append(
                HeaderName::from_bytes(name.as_bytes()).map_err(|_| {
                    Raise::<B>::new(ExceptionClass::builtin("ValueError"))
                        .arg(format!("invalid header name: {name}"))
                })?,
                HeaderValue::from_str(&value).map_err(|_| {
                    Raise::<B>::new(ExceptionClass::builtin("ValueError"))
                        .arg(format!("invalid header value for {name}"))
                })?,
            );
        }

        Ok(map)
    }

    fn all(&self, name: &HeaderName) -> Vec<String> {
        self.inner
            .get_all(name)
            .iter()
            .map(Self::latin1)
            .collect()
    }

    fn latin1(value: &HeaderValue) -> String {
        value
            .as_bytes()
            .iter()
            .map(|byte| char::from(*byte))
            .collect()
    }
}

#[host_class(
    backend = B,
    extends("collections.abc:Mapping"),
    crate_path = agentc_executor_python::guestpy
)]
impl<B: ExecutorBackend> Headers<B> {
    #[guestpy(constructor)]
    fn new(headers: Option<HeaderTypes<B>>) -> Result<Self, Error> {
        Ok(Self {
            inner: match headers {
                Some(headers) => Self::map(headers)?,
                None => HeaderMap::new(),
            },
            _marker: PhantomData,
        })
    }

    #[guestpy(dunder = "__getitem__")]
    fn get_item(&self, key: String) -> Result<String, Error> {
        let values = HeaderName::from_bytes(key.as_bytes())
            .ok()
            .map(|name| self.all(&name))
            .filter(|values| !values.is_empty());

        match values {
            Some(values) => Ok(values.join(", ")),
            None => Err(Raise::<B>::new(ExceptionClass::builtin("KeyError"))
                .arg(key)
                .into()),
        }
    }

    #[guestpy(dunder = "__iter__")]
    fn iter(&self) -> Result<HostIter<String>, Error> {
        Ok(HostIter::new(
            self.inner
                .keys()
                .map(|name| Ok(name.as_str().to_owned()))
                .collect::<Vec<_>>()
                .into_iter(),
        ))
    }

    #[guestpy(dunder = "__len__")]
    fn len(&self) -> Result<usize, Error> {
        Ok(self.inner.keys_len())
    }

    #[guestpy(dunder = "__contains__")]
    fn contains(&self, key: Object<B>) -> Result<bool, Error> {
        Ok(key
            .cast::<String>()
            .ok()
            .and_then(|key| HeaderName::from_bytes(key.as_bytes()).ok())
            .is_some_and(|name| self.inner.contains_key(&name)))
    }

    #[guestpy(method)]
    fn get_list(&self, key: String) -> Result<Vec<String>, Error> {
        Ok(HeaderName::from_bytes(key.as_bytes())
            .map(|name| self.all(&name))
            .unwrap_or_default())
    }

    #[guestpy(method)]
    fn multi_items(&self) -> Result<Vec<(String, String)>, Error> {
        Ok(self
            .inner
            .iter()
            .map(|(name, value)| (name.as_str().to_owned(), Self::latin1(value)))
            .collect())
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

    use super::Headers;

    const SOURCE: &str = r#"
from agentc_http import Headers


def inspect_headers():
    repeated = Headers([("set-cookie", "a"), ("Set-Cookie", "b")])
    mapping = Headers({"A": "1", "B": "2"})

    try:
        Headers({})["nope"]
    except KeyError as error:
        missing = error.args[0]

    try:
        Headers({"in valid": "1"})
    except ValueError as error:
        invalid = error.args[0]

    return [
        repeated["Set-Cookie"],
        repeated.get_list("set-cookie"),
        repeated.multi_items(),
        "SET-COOKIE" in repeated,
        42 in repeated,
        "in valid" in repeated,
        list(mapping),
        sorted(mapping.items()),
        mapping.get("c", "?"),
        len(mapping),
        Headers(repeated).get_list("set-cookie"),
        missing,
        invalid,
    ]
"#;

    fn module() -> Result<ModuleSpec<RustPython>, Error> {
        ModuleSpec::new("agentc_http").class::<Headers<RustPython>>()
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
    async fn headers_support_the_mapping_protocol() {
        let executor = executor().await;

        assert_eq!(
            call::<agentc_executor_python::json::Json>(&executor, "inspect_headers")
                .await
                .into_inner(),
            serde_json::json!([
                "a, b",
                ["a", "b"],
                [["set-cookie", "a"], ["set-cookie", "b"]],
                true,
                false,
                false,
                ["a", "b"],
                [["a", "1"], ["b", "2"]],
                "?",
                2,
                ["a", "b"],
                "nope",
                "invalid header name: in valid",
            ]),
        );

        executor
            .shutdown()
            .await
            .expect("executor shuts down");
    }
}
