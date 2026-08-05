// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::error::Error as StdError;

use reqwest::Error as ReqwestError;
use reqwest_middleware::Error as MiddlewareError;
use thiserror::Error;

use crate::client::policy::Denied;

/// An error produced by [`HttpClient`](crate::client::client::HttpClient).
#[derive(Debug, Error)]
pub enum HttpClientError {
    /// A [`Policy`](crate::client::policy::Policy) refused the request, a redirect hop, or the
    /// response.
    #[error("denied by policy '{policy}': {reason}")]
    Denied { policy: &'static str, reason: String },

    /// The request exceeded its deadline.
    #[error("request timed out")]
    Timeout,

    /// The response body exceeded
    /// [`max_response_bytes`](crate::client::builder::HttpClientBuilder::max_response_bytes).
    #[error("response body exceeded the limit of {limit} bytes")]
    BodyTooLarge { limit: u64 },

    /// The request exceeded
    /// [`max_redirects`](crate::client::builder::HttpClientBuilder::max_redirects).
    #[error("too many redirects")]
    TooManyRedirects,

    /// The request could not be built.
    #[error("invalid request: {message}")]
    InvalidRequest { message: String },

    /// The client could not be built.
    #[error("invalid configuration: {message}")]
    Configuration { message: String },

    /// The request failed below the policy layer.
    #[error("transport error: {source}")]
    Transport {
        #[source]
        source: MiddlewareError,
    },

    /// The response body could not be interpreted as the requested type.
    #[error("failed to decode the response: {message}")]
    Decode { message: String },
}

impl HttpClientError {
    /// Builds a [`Denied`](crate::client::errors::HttpClientError::Denied) error.
    pub fn denied(policy: &'static str, denial: Denied) -> Self {
        Self::Denied {
            policy,
            reason: denial.into_reason(),
        }
    }

    /// Builds an [`InvalidRequest`](crate::client::errors::HttpClientError::InvalidRequest) error.
    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self::InvalidRequest {
            message: message.into(),
        }
    }

    /// Builds a [`Configuration`](crate::client::errors::HttpClientError::Configuration) error.
    pub fn configuration(message: impl Into<String>) -> Self {
        Self::Configuration {
            message: message.into(),
        }
    }

    /// Builds a [`Decode`](crate::client::errors::HttpClientError::Decode) error.
    pub fn decode(message: impl Into<String>) -> Self {
        Self::Decode {
            message: message.into(),
        }
    }
}

/// The reason a redirect hop was rejected.
///
/// `reqwest` follows redirects below the middleware layer and boxes whatever
/// [`Attempt::error`](reqwest::redirect::Attempt::error) receives, so a rejection is recovered from
/// the resulting error's source chain rather than returned directly.
#[derive(Debug, Error)]
pub(crate) enum RedirectRejection {
    #[error("denied by policy '{policy}': {reason}")]
    Denied { policy: &'static str, reason: String },

    #[error("too many redirects")]
    LimitExceeded,
}

impl RedirectRejection {
    fn recover(error: &ReqwestError) -> Option<Self> {
        let mut source = error.source();

        while let Some(current) = source {
            if let Some(rejection) = current.downcast_ref::<Self>() {
                return Some(match rejection {
                    Self::Denied { policy, reason } => Self::Denied {
                        policy,
                        reason: reason.clone(),
                    },
                    Self::LimitExceeded => Self::LimitExceeded,
                });
            }

            source = current.source();
        }

        None
    }
}

impl From<RedirectRejection> for HttpClientError {
    fn from(rejection: RedirectRejection) -> Self {
        match rejection {
            RedirectRejection::Denied { policy, reason } => Self::Denied { policy, reason },
            RedirectRejection::LimitExceeded => Self::TooManyRedirects,
        }
    }
}

impl From<MiddlewareError> for HttpClientError {
    fn from(error: MiddlewareError) -> Self {
        match error {
            // `PolicyMiddleware` boxes this crate's own error, so recover it rather than
            // reporting a denial as an opaque transport failure.
            MiddlewareError::Middleware(error) => error
                .downcast::<Self>()
                .unwrap_or_else(|error| Self::Transport {
                    source: MiddlewareError::Middleware(error),
                }),
            MiddlewareError::Reqwest(error) => match RedirectRejection::recover(&error) {
                Some(rejection) => rejection.into(),
                None if error.is_timeout() => Self::Timeout,
                None => Self::Transport {
                    source: MiddlewareError::Reqwest(error),
                },
            },
        }
    }
}
