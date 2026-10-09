// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::fmt::{Display, Formatter, Result as FmtResult};

/// Identity for a provider kind.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ProviderKind(&'static str);

impl ProviderKind {
    pub const fn new(value: &'static str) -> Self {
        Self(value)
    }

    pub const fn as_str(&self) -> &'static str {
        self.0
    }
}

impl Display for ProviderKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        self.as_str().fmt(f)
    }
}

/// Identity for a provider instance.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProviderId(String);

impl ProviderId {
    pub fn new(value: String) -> Self {
        Self(value)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_str(self) -> String {
        self.0
    }
}

impl Display for ProviderId {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        self.as_str().fmt(f)
    }
}

impl From<&str> for ProviderId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for ProviderId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&String> for ProviderId {
    fn from(s: &String) -> Self {
        Self(s.clone())
    }
}

impl From<ProviderId> for String {
    fn from(s: ProviderId) -> Self {
        s.into_str()
    }
}

/// Identifies a model
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModelId(String);

impl ModelId {
    pub fn new(value: String) -> Self {
        Self(value)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_str(self) -> String {
        self.0
    }
}

impl Display for ModelId {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        self.as_str().fmt(f)
    }
}

impl From<&str> for ModelId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for ModelId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&String> for ModelId {
    fn from(s: &String) -> Self {
        Self(s.clone())
    }
}

impl From<ModelId> for String {
    fn from(s: ModelId) -> Self {
        s.into_str()
    }
}
