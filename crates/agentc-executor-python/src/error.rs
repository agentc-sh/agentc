// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

/// Errors produced while constructing, operating, or shutting down an executor.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An operation failed inside GuestPy.
    #[error(transparent)]
    Guest(#[from] guestpy::errors::Error),

    /// An unexpected error occurred.
    #[error("unexpected error: {message}")]
    Unexpected {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
}

impl Error {
    /// Creates an [`Error::Guest`] error.
    pub fn guest(error: impl Into<guestpy::errors::Error>) -> Self {
        Self::Guest(error.into())
    }

    /// Creates an [`Error::Unexpected`] error.
    pub fn unexpected(
        message: impl Into<String>,
        source: impl Into<Option<Box<dyn std::error::Error + Send + Sync>>>,
    ) -> Self {
        Self::Unexpected {
            message: message.into(),
            source: source.into(),
        }
    }
}
