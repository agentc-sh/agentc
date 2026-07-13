// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::time::Duration;
use thiserror::Error;

use crate::types::identity::{ModelId, ProviderId};

/// Errors that can occur during model client construction or completion.
#[derive(Debug, Error)]
pub enum ModelError {
    /// An error returned by the provider during a completion request.
    #[error("provider '{provider}' error: {message}")]
    Provider {
        provider: ProviderId,
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// An error that occurred while consuming the completion stream.
    #[error("stream error: {message}")]
    Stream {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// A transient provider condition (rate limiting, overload, or a server
    /// error) that may succeed if the request is retried.
    #[error("provider '{provider}' transient error: {message}")]
    Transient {
        provider: ProviderId,
        message: String,
        retry_after: Option<Duration>,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// The request handshake did not complete within the allotted time.
    #[error("provider '{provider}' timed out after {elapsed:?}")]
    Timeout {
        provider: ProviderId,
        elapsed: Duration,
    },

    /// An error in the client or factory configuration.
    #[error("configuration error: {message}")]
    Configuration { message: String },

    /// A serialization or deserialization error from serde_json.
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// An invalid request, such as missing required fields or invalid parameter values.
    #[error("invalid request: {message}")]
    InvalidRequest { message: String },

    /// A model was requested that is not allowed by the provider's constraints.
    #[error("model '{model}' is not allowed for provider '{provider}'")]
    ModelNotAllowed {
        provider: ProviderId,
        model: ModelId,
    },

    /// The requested feature is not supported by the given provider.
    #[error("unsupported feature '{feature}' for provider '{provider}'")]
    UnsupportedFeature {
        provider: ProviderId,
        feature: String,
    },

    /// No factory has been registered for the given provider.
    #[error("no factory registered for provider '{provider}'")]
    UnknownProvider { provider: ProviderId },
}

impl ModelError {
    /// The short, bounded `error.type` value for this error.
    pub fn error_type(&self) -> &'static str {
        match self {
            Self::Provider { .. } => "provider",
            Self::Stream { .. } => "stream",
            Self::Transient { .. } => "transient",
            Self::Timeout { .. } => "timeout",
            Self::Configuration { .. } => "configuration",
            Self::Serialization(_) => "serialization",
            Self::InvalidRequest { .. } => "invalid_request",
            Self::ModelNotAllowed { .. } => "model_not_allowed",
            Self::UnsupportedFeature { .. } => "unsupported_feature",
            Self::UnknownProvider { .. } => "unknown_provider",
        }
    }

    /// Creates a [`ModelError::Provider`] with an optional source error.
    pub fn provider<E>(
        provider: impl Into<ProviderId>,
        message: impl Into<String>,
        source: Option<E>,
    ) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::Provider {
            provider: provider.into(),
            message: message.into(),
            source: source.map(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>),
        }
    }

    /// Creates a [`ModelError::Stream`] with an optional source error.
    pub fn stream<E>(message: impl Into<String>, source: Option<E>) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::Stream {
            message: message.into(),
            source: source.map(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>),
        }
    }

    /// Creates a [`ModelError::Transient`] with an optional retry-after hint and
    /// source error.
    pub fn transient<E>(
        provider: impl Into<ProviderId>,
        message: impl Into<String>,
        retry_after: Option<Duration>,
        source: Option<E>,
    ) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::Transient {
            provider: provider.into(),
            message: message.into(),
            retry_after,
            source: source.map(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>),
        }
    }

    /// Creates a [`ModelError::Timeout`].
    pub fn timeout(provider: impl Into<ProviderId>, elapsed: Duration) -> Self {
        Self::Timeout {
            provider: provider.into(),
            elapsed,
        }
    }

    /// Whether this error may succeed if the handshake is retried. True for
    /// transient provider conditions and handshake timeouts.
    pub fn is_transient(&self) -> bool {
        matches!(self, Self::Transient { .. } | Self::Timeout { .. })
    }

    /// A provider-supplied hint for how long to wait before retrying, when present.
    pub fn retry_after(&self) -> Option<Duration> {
        match self {
            Self::Transient { retry_after, .. } => *retry_after,
            _ => None,
        }
    }

    /// Creates a [`ModelError::Configuration`].
    pub fn configuration(message: impl Into<String>) -> Self {
        Self::Configuration { message: message.into() }
    }

    /// Creates a [`ModelError::InvalidRequest`].
    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self::InvalidRequest { message: message.into() }
    }

    /// Creates a [`ModelError::ModelNotAllowed`].
    pub fn model_not_allowed(provider: impl Into<ProviderId>, model: impl Into<ModelId>) -> Self {
        Self::ModelNotAllowed {
            provider: provider.into(),
            model: model.into(),
        }
    }

    /// Creates a [`ModelError::UnsupportedFeature`].
    pub fn unsupported_feature(
        provider: impl Into<ProviderId>,
        feature: impl Into<String>,
    ) -> Self {
        Self::UnsupportedFeature {
            provider: provider.into(),
            feature: feature.into(),
        }
    }

    /// Creates a [`ModelError::UnknownProvider`].
    pub fn unknown_provider(provider: impl Into<ProviderId>) -> Self {
        Self::UnknownProvider { provider: provider.into() }
    }
}
