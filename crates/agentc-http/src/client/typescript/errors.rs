// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use agentc_executor_typescript::guestjs::errors::Error;

use crate::client::errors::HttpClientError;

impl From<HttpClientError> for Error {
    fn from(error: HttpClientError) -> Self {
        Error::sourced_unexpected(format!("agentc:http: {error}"), Some(error))
    }
}
