// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use reqwest::header::{InvalidHeaderName, InvalidHeaderValue};

#[derive(Debug, thiserror::Error)]
pub enum A2aClientError {
    #[error("invalid A2A client configuration: {0}")]
    Configuration(String),

    #[error("A2A request failed: {0}")]
    Request(#[from] reqwest_middleware::Error),

    #[error("A2A response error: status {status}, body: {body}")]
    Response {
        status: reqwest::StatusCode,
        body: String,
    },

    #[error("failed to deserialize A2A response: {0}")]
    Decode(#[from] reqwest::Error),

    #[error("failed to deserialize A2A stream event: {0}")]
    StreamDecode(String),

    #[error("serialization error: {0}")]
    Serialize(#[from] serde_json::Error),
}

impl A2aClientError {
    pub fn configuration(message: impl Into<String>) -> Self {
        A2aClientError::Configuration(message.into())
    }

    pub fn request(error: impl Into<reqwest_middleware::Error>) -> Self {
        A2aClientError::Request(error.into())
    }

    pub fn response(status: impl Into<reqwest::StatusCode>, body: impl Into<String>) -> Self {
        A2aClientError::Response {
            status: status.into(),
            body: body.into(),
        }
    }

    pub fn decode(error: impl Into<reqwest::Error>) -> Self {
        A2aClientError::Decode(error.into())
    }

    pub fn stream_decode(message: impl Into<String>) -> Self {
        A2aClientError::StreamDecode(message.into())
    }

    pub fn serialize(error: impl Into<serde_json::Error>) -> Self {
        A2aClientError::Serialize(error.into())
    }
}

impl From<InvalidHeaderName> for A2aClientError {
    fn from(error: InvalidHeaderName) -> Self {
        A2aClientError::configuration(format!("invalid header name: {}", error))
    }
}

impl From<InvalidHeaderValue> for A2aClientError {
    fn from(error: InvalidHeaderValue) -> Self {
        A2aClientError::configuration(format!("invalid header value: {}", error))
    }
}
