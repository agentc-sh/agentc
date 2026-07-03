// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use anyhow::Error as AnyhowError;
use sea_orm::{DbErr, SqlErr};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] DbErr),
    #[error("SQL error: {0}")]
    SqlError(#[from] SqlErr),
    #[error("Transaction error: {0}")]
    TransactionError(#[from] std::io::Error),
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    #[error("Missing primary DSN")]
    MissingPrimaryDsn,
    #[error("Unknown field name: {0}")]
    UnknownFieldName(String),
    #[error("Unexpected error: {0}")]
    UnexpectedError(String),
}

impl DatabaseError {
    pub fn invalid_input(msg: impl Into<String>) -> Self {
        DatabaseError::InvalidInput(msg.into())
    }

    pub fn unknown_field_name(field_name: impl Into<String>) -> Self {
        DatabaseError::UnknownFieldName(field_name.into())
    }

    pub fn unexpected_error(msg: impl Into<String>) -> Self {
        DatabaseError::UnexpectedError(msg.into())
    }
}

impl From<String> for DatabaseError {
    fn from(err: String) -> Self {
        DatabaseError::UnexpectedError(err)
    }
}

impl From<AnyhowError> for DatabaseError {
    fn from(err: AnyhowError) -> Self {
        DatabaseError::UnexpectedError(err.to_string())
    }
}
