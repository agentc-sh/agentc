// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::collections::{BTreeMap, BTreeSet};

use sanitizer::prelude::*;
use serde::{Deserialize, Serialize};
use validator::Validate;

use agentc_blocks::types::RuntimeValue;

#[derive(Debug, Clone, Serialize, Deserialize, Validate, Sanitizer)]
#[serde(default)]
pub struct ManifestNetwork {
    /// The `User-Agent` header sent with every request, if any.
    pub user_agent: RuntimeValue<Option<String>>,
    /// Additional headers sent with every request.
    pub headers: RuntimeValue<BTreeMap<String, String>>,
    /// Connection, timeout, and size limits.
    #[validate(nested)]
    pub limits: ManifestNetworkLimits,
    /// The egress policy applied to every request.
    #[validate(nested)]
    pub policy: ManifestNetworkPolicy,
}

impl Default for ManifestNetwork {
    fn default() -> Self {
        Self {
            user_agent: RuntimeValue::default_runtime("NETWORK_USER_AGENT", None),
            headers: RuntimeValue::default_runtime("NETWORK_HEADERS", BTreeMap::new()),
            limits: ManifestNetworkLimits::default(),
            policy: ManifestNetworkPolicy::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, Sanitizer)]
#[serde(default)]
pub struct ManifestNetworkLimits {
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

impl Default for ManifestNetworkLimits {
    fn default() -> Self {
        Self {
            connect_timeout_ms: RuntimeValue::default_runtime("NETWORK_CONNECT_TIMEOUT_MS", None),
            read_timeout_ms: RuntimeValue::default_runtime("NETWORK_READ_TIMEOUT_MS", None),
            request_timeout_ms: RuntimeValue::default_runtime("NETWORK_REQUEST_TIMEOUT_MS", None),
            max_redirects: RuntimeValue::default_runtime("NETWORK_MAX_REDIRECTS", 5usize),
            max_response_bytes: RuntimeValue::default_runtime("NETWORK_MAX_RESPONSE_BYTES", None),
            concurrency_limit: RuntimeValue::default_runtime("NETWORK_CONCURRENCY_LIMIT", None),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, Sanitizer)]
#[serde(default)]
pub struct ManifestNetworkPolicy {
    /// Which categories of IP address outbound requests may reach.
    #[validate(nested)]
    pub addresses: ManifestNetworkPolicyAddresses,
    /// The HTTP methods outbound requests may use. `None` means unrestricted.
    pub methods: RuntimeValue<Option<BTreeSet<String>>>,
    /// URL patterns outbound requests must match. Empty means unrestricted.
    pub allow: RuntimeValue<Vec<ManifestNetworkUrlPattern>>,
}

impl Default for ManifestNetworkPolicy {
    fn default() -> Self {
        Self {
            addresses: ManifestNetworkPolicyAddresses::default(),
            methods: RuntimeValue::default_runtime("NETWORK_ALLOW_METHODS", None),
            allow: RuntimeValue::default_runtime("NETWORK_ALLOW_PATTERNS", Vec::new()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, Sanitizer)]
#[serde(default)]
pub struct ManifestNetworkPolicyAddresses {
    /// Additionally permits loopback addresses.
    pub allow_loopback: RuntimeValue<bool>,
    /// Additionally permits private addresses.
    pub allow_private: RuntimeValue<bool>,
    /// Additionally permits link-local addresses, including the cloud metadata address.
    pub allow_link_local: RuntimeValue<bool>,
}

impl Default for ManifestNetworkPolicyAddresses {
    fn default() -> Self {
        Self {
            allow_loopback: RuntimeValue::default_runtime("NETWORK_ALLOW_LOOPBACK", false),
            allow_private: RuntimeValue::default_runtime("NETWORK_ALLOW_PRIVATE", false),
            allow_link_local: RuntimeValue::default_runtime("NETWORK_ALLOW_LINK_LOCAL", false),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestNetworkUrlPattern {
    pub protocol: Option<String>,
    pub hostname: Option<String>,
    pub port: Option<String>,
    pub pathname: Option<String>,
}
