// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use agentc_executor_typescript::guestjs::{errors::Error, host::iter::Iterable, host_class};
use http::HeaderMap;

/// The guest-visible header collection on a
/// [`Response`](crate::client::typescript::response::Response).
pub struct Headers {
    inner: HeaderMap,
}

impl Headers {
    pub(crate) fn new(inner: impl Into<HeaderMap>) -> Self {
        Self { inner: inner.into() }
    }

    fn lookup(&self, name: &str) -> Option<String> {
        self.inner
            .get(name)
            .and_then(|value| value.to_str().ok())
            .map(ToOwned::to_owned)
    }

    fn names(&self) -> Vec<String> {
        self.inner
            .keys()
            .map(|name| name.as_str().to_owned())
            .collect()
    }

    fn pairs(&self) -> Vec<Vec<String>> {
        self.inner
            .iter()
            .filter_map(|(name, value)| {
                Some(vec![name.as_str().to_owned(), value.to_str().ok()?.to_owned()])
            })
            .collect()
    }
}

#[host_class(crate_path = agentc_executor_typescript::guestjs)]
impl Headers {
    #[guestjs(method)]
    fn get(&self, name: String) -> Result<Option<String>, Error> {
        Ok(self.lookup(&name))
    }

    #[guestjs(method)]
    fn has(&self, name: String) -> Result<bool, Error> {
        Ok(self.lookup(&name).is_some())
    }

    #[guestjs(method)]
    fn keys(&self) -> Result<Vec<String>, Error> {
        Ok(self.names())
    }

    #[guestjs(method)]
    fn entries(&self) -> Result<Vec<Vec<String>>, Error> {
        Ok(self.pairs())
    }

    #[guestjs(method, symbol = "iterator")]
    fn iterator(&self) -> Result<Iterable<impl Iterator<Item = Vec<String>> + use<>>, Error> {
        Ok(Iterable::new(self.pairs().into_iter()))
    }
}
