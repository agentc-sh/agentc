// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use thiserror::Error;

#[derive(Debug, Error)]
pub enum BlocksError {
    /// The manifest's `build.archetype` field names a template that has not
    /// been registered with the [`ArchetypeResolver`](crate::archetype::resolver::ArchetypeResolver).
    #[error("unknown archetype: {0:?}")]
    UnknownArchetype(String),

    /// The manifest's selected graph has not been registered with the
    /// [`GraphResolver`](crate::graph::resolver::GraphResolver).
    #[error("unknown graph: {0:?}")]
    UnknownGraph(String),

    /// The manifest's selected protocol has not been registered with the
    /// [`ProtocolResolver`](crate::protocol::resolver::ProtocolResolver).
    #[error("unknown protocol: {0:?}")]
    UnknownProtocol(String),

    /// A compiler component was registered more than once under the same name.
    #[error("duplicate {component} registration: {name:?}")]
    DuplicateRegistration {
        component: &'static str,
        name: String,
    },

    /// A required manifest field was absent and has no default.
    #[error("missing required manifest field: {field}")]
    MissingField { field: &'static str },

    /// The manifest contained a logically invalid combination of fields.
    #[error("invalid manifest: {reason}")]
    InvalidManifest { reason: String },

    /// An error occurred during archetype resolution.
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// An unexpected error.
    #[error("unexpected error: {message}")]
    Unexpected {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
}

impl BlocksError {
    pub fn missing(field: &'static str) -> Self {
        Self::MissingField { field }
    }

    pub fn invalid(reason: impl Into<String>) -> Self {
        Self::InvalidManifest { reason: reason.into() }
    }

    pub fn duplicate_registration(component: &'static str, name: impl Into<String>) -> Self {
        Self::DuplicateRegistration { component, name: name.into() }
    }

    pub fn unexpected(message: impl Into<String>) -> Self {
        Self::Unexpected { message: message.into(), source: None }
    }

    pub fn sourced_unexpected(
        message: impl Into<String>,
        source: Option<impl Into<Box<dyn std::error::Error + Send + Sync>>>,
    ) -> Self {
        Self::Unexpected {
            message: message.into(),
            source: source.map(|s| s.into()),
        }
    }
}
