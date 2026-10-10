// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::sync::Arc;

use reqwest::dns::{Addrs, Name, Resolve, Resolving};
use tokio::net::lookup_host;

use crate::client::{errors::HttpClientError, policy::Policy};

pub(crate) struct GuardedResolver {
    policies: Arc<[Arc<dyn Policy>]>,
}

impl GuardedResolver {
    pub(crate) fn new(policies: Arc<[Arc<dyn Policy>]>) -> Self {
        Self { policies }
    }
}

impl Resolve for GuardedResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let policies = self.policies.clone();
        let host = name.as_str().to_owned();

        Box::pin(async move {
            let mut rejection = None;

            // Port zero is replaced by the scheme's conventional port after resolution, so a
            // policy sees the address that will actually be connected to.
            let permitted = lookup_host((host.as_str(), 0))
                .await?
                .filter(|address| {
                    match policies.iter().find_map(|policy| {
                        policy
                            .check_address(&host, *address)
                            .err()
                            .map(|denial| HttpClientError::denied(policy.name(), denial))
                    }) {
                        Some(denied) => {
                            rejection.get_or_insert(denied);

                            false
                        }
                        None => true,
                    }
                })
                .collect::<Vec<_>>();

            match (permitted.is_empty(), rejection) {
                (true, Some(rejection)) => Err(Box::new(rejection).into()),
                _ => Ok(Box::new(permitted.into_iter()) as Addrs),
            }
        })
    }
}
