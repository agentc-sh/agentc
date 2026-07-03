// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AssetError {
    /// No registered handler was able to handle the given URI.
    #[error("no handler found for source: '{uri}'")]
    NoHandler { uri: String },

    /// An I/O error occurred while writing the artifact to the store.
    #[error("I/O error while fetching '{uri}': {source}")]
    Io {
        uri: String,
        #[source]
        source: std::io::Error,
    },

    /// A remote handler failed to download the artifact.
    #[error("failed to download '{uri}': {message}")]
    Download { uri: String, message: String },
}

impl AssetError {
    pub fn no_handler(uri: impl Into<String>) -> Self {
        Self::NoHandler { uri: uri.into() }
    }

    pub fn io(uri: impl Into<String>, source: std::io::Error) -> Self {
        Self::Io { uri: uri.into(), source }
    }

    pub fn download(uri: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Download { uri: uri.into(), message: message.into() }
    }
}
