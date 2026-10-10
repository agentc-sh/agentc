// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use agentc_executor_typescript::guestjs::host::{ClassSpec, HostClass};
use http::HeaderMap;

/// The guest-visible header collection on a
/// [`Response`](crate::client::typescript::response::Response).
pub struct Headers {
    inner: HeaderMap,
}

impl Headers {
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

    pub(crate) fn new(inner: impl Into<HeaderMap>) -> Self {
        Self { inner: inner.into() }
    }
}

impl HostClass for Headers {
    const NAME: &'static str = "Headers";

    fn build(spec: &mut ClassSpec<Self>) {
        spec.method("get", |headers, scope, args| {
            Ok(headers.lookup(&args.get_owned::<String>(scope, 0)?))
        });

        spec.method("has", |headers, scope, args| {
            Ok(headers
                .lookup(&args.get_owned::<String>(scope, 0)?)
                .is_some())
        });

        spec.method("keys", |headers, _scope, _args| Ok(headers.names()));

        spec.method("entries", |headers, _scope, _args| Ok(headers.pairs()));

        spec.iterable(|headers, _scope| Ok(headers.pairs()));
    }
}
