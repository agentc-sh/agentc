// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use rig_core::{completion::CompletionError, http_client};

use crate::{
    errors::{IntoModelError, ModelError},
    types::identity::ProviderId,
};

/// Case-insensitive substrings marking a `rig` provider error message as a
/// transient condition worth retrying. `rig` stringifies the HTTP status for
/// streaming errors, so `"invalid status code: 5"` catches any stringified 5xx.
const TRANSIENT_MARKERS: &[&str] = &[
    "429",
    "too many requests",
    "rate limit",
    "overloaded",
    "overload",
    "invalid status code: 5",
    "internal server error",
    "service unavailable",
    "bad gateway",
    "gateway timeout",
    "server error",
    "timeout",
    "timed out",
    "connection",
    "temporarily",
    "unavailable",
    "try again",
];

impl IntoModelError for CompletionError {
    fn into_model_error(self, provider: impl Into<ProviderId>) -> ModelError {
        let transient = match &self {
            CompletionError::HttpError(
                http_client::Error::InvalidStatusCode(code)
                | http_client::Error::InvalidStatusCodeWithMessage(code, _),
            ) => code.as_u16() == 429 || code.is_server_error(),
            CompletionError::HttpError(
                http_client::Error::Instance(_) | http_client::Error::StreamEnded,
            ) => true,
            CompletionError::ProviderError(message) | CompletionError::ResponseError(message) => {
                let message = message.to_lowercase();

                TRANSIENT_MARKERS
                    .iter()
                    .any(|&marker| message.contains(marker))
            }
            _ => false,
        };

        if transient {
            ModelError::transient(provider, self.to_string(), None, Some(self))
        } else {
            ModelError::provider(provider, self.to_string(), Some(self))
        }
    }
}
