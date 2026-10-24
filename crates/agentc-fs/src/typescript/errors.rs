// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use agentc_executor_typescript::guestjs::errors::Error;

impl From<crate::errors::Error> for Error {
    fn from(error: crate::errors::Error) -> Self {
        Error::sourced_unexpected(format!("agentc:fs: {error}"), error)
    }
}
