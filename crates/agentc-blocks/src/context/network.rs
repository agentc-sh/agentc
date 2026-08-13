// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::types::RuntimeValue;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResolvedContextNetwork {
    /// The `User-Agent` header sent with every request, if any.
    pub user_agent: RuntimeValue<Option<String>>,
    /// Additional headers sent with every request.
    pub headers: RuntimeValue<BTreeMap<String, String>>,
    /// Connection, timeout, and size limits.
    pub limits: ResolvedContextNetworkLimits,
    /// The egress policy applied to every request.
    pub policy: ResolvedContextNetworkPolicy,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResolvedContextNetworkLimits {
    /// The maximum time spent establishing a connection, in milliseconds.
    pub connect_timeout_ms: RuntimeValue<Option<u64>>,
    /// The maximum time between response body chunks, in milliseconds.
    pub read_timeout_ms: RuntimeValue<Option<u64>>,
    /// A deadline for the whole request, in milliseconds.
    pub request_timeout_ms: RuntimeValue<Option<u64>>,
    /// How many redirects one request may follow.
    pub max_redirects: RuntimeValue<usize>,
    /// How many response body bytes are accepted.
    pub max_response_bytes: RuntimeValue<Option<u64>>,
    /// How many requests this client may have in flight at once.
    pub concurrency_limit: RuntimeValue<Option<usize>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResolvedContextNetworkPolicy {
    /// Which categories of IP address outbound requests may reach.
    pub addresses: ResolvedContextNetworkPolicyAddresses,
    /// The HTTP methods outbound requests may use. `None` means unrestricted.
    pub methods: RuntimeValue<Option<BTreeSet<String>>>,
    /// URL patterns outbound requests must match. Empty means unrestricted.
    pub allow: RuntimeValue<Vec<ResolvedContextNetworkUrlPattern>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResolvedContextNetworkPolicyAddresses {
    /// Additionally permits loopback addresses.
    pub allow_loopback: RuntimeValue<bool>,
    /// Additionally permits private addresses.
    pub allow_private: RuntimeValue<bool>,
    /// Additionally permits link-local addresses, including the cloud metadata address.
    pub allow_link_local: RuntimeValue<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedContextNetworkUrlPattern {
    pub protocol: Option<String>,
    pub hostname: Option<String>,
    pub port: Option<String>,
    pub pathname: Option<String>,
}
