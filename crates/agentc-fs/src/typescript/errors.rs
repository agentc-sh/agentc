// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use agentc_executor_typescript::guestjs::errors::Error;

use crate::errors::Error as FsError;

impl From<FsError> for Error {
    fn from(error: FsError) -> Self {
        Error::sourced_unexpected(format!("agentc:fs: {error}"), Some(error))
    }
}
