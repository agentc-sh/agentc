// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::sync::Arc;

use reqwest::dns::{Addrs, Name, Resolve, Resolving};
use tokio::net::lookup_host;

use crate::client::{errors::HttpClientError, policy::AddressFilter};

pub(crate) struct GuardedResolver {
    filter: Arc<dyn AddressFilter>,
}

impl GuardedResolver {
    pub(crate) fn new(filter: Arc<dyn AddressFilter>) -> Self {
        Self { filter }
    }
}

impl Resolve for GuardedResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let filter = self.filter.clone();
        let host = name.as_str().to_owned();

        Box::pin(async move {
            // Port zero is replaced by the scheme's conventional port after resolution, so the
            // filter sees the address that will actually be connected to.
            let permitted = lookup_host((host.as_str(), 0))
                .await?
                .filter(|address| filter.allows(&host, *address))
                .collect::<Vec<_>>();

            if permitted.is_empty() {
                return Err(Box::new(HttpClientError::address_denied(host)).into());
            }

            Ok(Box::new(permitted.into_iter()) as Addrs)
        })
    }
}
