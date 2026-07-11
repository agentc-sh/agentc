// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::time::Duration;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

use crate::client::errors::A2aClientError;

#[derive(Debug, Clone)]
pub struct A2aClientConfig {
    pub base_url: String,
    pub default_headers: HeaderMap,
    pub timeout: Duration,
}

impl A2aClientConfig {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url
                .into()
                .trim_end_matches('/')
                .to_string(),
            default_headers: HeaderMap::new(),
            timeout: Duration::from_secs(30),
        }
    }

    pub fn header(mut self, key: impl AsRef<str>, value: impl AsRef<str>) -> Self {
        self.default_headers.insert(
            HeaderName::from_bytes(key.as_ref().as_bytes()).unwrap(),
            HeaderValue::from_str(value.as_ref()).unwrap(),
        );
        self
    }

    pub fn headers(mut self, headers: impl IntoIterator<Item = (impl AsRef<str>, impl AsRef<str>)>) -> Self {
        for (key, value) in headers {
            self.default_headers.insert(
                HeaderName::from_bytes(key.as_ref().as_bytes()).unwrap(),
                HeaderValue::from_str(value.as_ref()).unwrap(),
            );
        }
        self
    }

    pub fn try_header(mut self, key: impl AsRef<str>, value: impl AsRef<str>) -> Result<Self, A2aClientError> {
        self.default_headers.insert(
            HeaderName::from_bytes(key.as_ref().as_bytes())?,
            HeaderValue::from_str(value.as_ref())?,
        );
        Ok(self)
    }

    pub fn try_headers(mut self, headers: impl IntoIterator<Item = (impl AsRef<str>, impl AsRef<str>)>) -> Result<Self, A2aClientError> {
        for (key, value) in headers {
            self.default_headers.insert(
                HeaderName::from_bytes(key.as_ref().as_bytes())?,
                HeaderValue::from_str(value.as_ref())?,
            );
        }
        Ok(self)
    }

    pub fn timeout(mut self, duration: Duration) -> Self {
        self.timeout = duration;
        self
    }
}
