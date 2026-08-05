// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::collections::BTreeSet;

use http::Method;

use crate::client::policy::{Denied, Policy, RequestContext};

/// Restricts requests to a set of methods.
pub struct MethodPolicy {
    methods: BTreeSet<String>,
}

impl MethodPolicy {
    /// Permits the given methods.
    pub fn allow<I, M>(methods: I) -> Self
    where
        I: IntoIterator<Item = M>,
        M: Into<Method>,
    {
        Self {
            methods: methods
                .into_iter()
                .map(|method| method.into().as_str().to_owned())
                .collect(),
        }
    }

    /// Permits `GET`, `HEAD`, and `POST`.
    pub fn safe() -> Self {
        Self::allow([Method::GET, Method::HEAD, Method::POST])
    }
}

impl Policy for MethodPolicy {
    fn name(&self) -> &'static str {
        "method-allowlist"
    }

    fn check_request(&self, request: &RequestContext<'_>) -> Result<(), Denied> {
        match self.methods.contains(request.method().as_str()) {
            true => Ok(()),
            false => Err(Denied::new(format!(
                "method {} is not permitted",
                request.method(),
            ))),
        }
    }
}
