// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::{
    error::Error as StdError,
    io::{Error as IoError, ErrorKind},
};

use crate::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("path not found: {0}")]
    NotFound(PathBuf),

    #[error("path already exists: {0}")]
    AlreadyExists(PathBuf),

    #[error("path is not a directory: {0}")]
    NotDirectory(PathBuf),

    #[error("path is a directory: {0}")]
    IsDirectory(PathBuf),

    #[error("permission denied: {0}")]
    PermissionDenied(PathBuf),

    #[error("operation is unsupported: {message}")]
    Unsupported { message: String },

    #[error("path is invalid: {message}")]
    InvalidPath { message: String },

    #[error("path escapes authority: {0}")]
    PathEscapesAuthority(PathBuf),

    #[error("cannot rename across mounted backends: {from} -> {to}")]
    CrossBackendRename { from: PathBuf, to: PathBuf },

    #[error("unexpected filesystem error: {message}")]
    Unexpected {
        message: String,
        #[source]
        source: Option<Box<dyn StdError + Send + Sync>>,
    },
}

impl Error {
    pub fn not_found(path: impl Into<PathBuf>) -> Self {
        Error::NotFound(path.into())
    }

    pub fn already_exists(path: impl Into<PathBuf>) -> Self {
        Error::AlreadyExists(path.into())
    }

    pub fn not_directory(path: impl Into<PathBuf>) -> Self {
        Error::NotDirectory(path.into())
    }

    pub fn is_directory(path: impl Into<PathBuf>) -> Self {
        Error::IsDirectory(path.into())
    }

    pub fn permission_denied(path: impl Into<PathBuf>) -> Self {
        Error::PermissionDenied(path.into())
    }

    pub fn unsupported(message: impl Into<String>) -> Self {
        Error::Unsupported { message: message.into() }
    }

    pub fn invalid_path(message: impl Into<String>) -> Self {
        Error::InvalidPath { message: message.into() }
    }

    pub fn path_escapes_authority(path: impl Into<PathBuf>) -> Self {
        Error::PathEscapesAuthority(path.into())
    }

    pub fn cross_backend_rename(from: impl Into<PathBuf>, to: impl Into<PathBuf>) -> Self {
        Error::CrossBackendRename { from: from.into(), to: to.into() }
    }

    pub fn unexpected(
        message: impl Into<String>,
        source: impl Into<Option<Box<dyn StdError + Send + Sync>>>,
    ) -> Self {
        Error::Unexpected {
            message: message.into(),
            source: source.into(),
        }
    }
}

pub(crate) trait IntoFsError {
    fn into_fs_error(self, path: &Path, message: &str) -> Error;
}

impl IntoFsError for IoError {
    fn into_fs_error(self, path: &Path, message: &str) -> Error {
        match self.kind() {
            ErrorKind::NotFound => Error::not_found(path),
            ErrorKind::AlreadyExists => Error::already_exists(path),
            ErrorKind::PermissionDenied => Error::permission_denied(path),
            ErrorKind::InvalidInput => Error::invalid_path(self.to_string()),
            _ => Error::unexpected(
                message,
                Some(Box::new(self) as Box<dyn StdError + Send + Sync>),
            ),
        }
    }
}

#[cfg(target_os = "linux")]
impl IntoFsError for rustix::io::Errno {
    fn into_fs_error(self, path: &Path, message: &str) -> Error {
        match self.kind() {
            ErrorKind::NotFound => Error::not_found(path),
            ErrorKind::AlreadyExists => Error::already_exists(path),
            ErrorKind::PermissionDenied => Error::permission_denied(path),
            ErrorKind::InvalidInput => Error::invalid_path(self.to_string()),
            _ => Error::unexpected(
                message,
                Some(Box::new(self) as Box<dyn StdError + Send + Sync>),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{errors::Error, path::PathBuf};

    #[test]
    fn path_constructor_variants_store_paths() {
        assert!(matches!(
            Error::not_found(PathBuf::parse("/missing").unwrap()),
            Error::NotFound(path) if path.to_string_lossy() == "/missing"
        ));
        assert!(matches!(
            Error::already_exists(PathBuf::parse("/existing").unwrap()),
            Error::AlreadyExists(path) if path.to_string_lossy() == "/existing"
        ));
        assert!(matches!(
            Error::not_directory(PathBuf::parse("/file").unwrap()),
            Error::NotDirectory(path) if path.to_string_lossy() == "/file"
        ));
        assert!(matches!(
            Error::is_directory(PathBuf::parse("/dir").unwrap()),
            Error::IsDirectory(path) if path.to_string_lossy() == "/dir"
        ));
        assert!(matches!(
            Error::permission_denied(PathBuf::parse("/private").unwrap()),
            Error::PermissionDenied(path) if path.to_string_lossy() == "/private"
        ));
        assert!(matches!(
            Error::path_escapes_authority(PathBuf::parse("../escape").unwrap()),
            Error::PathEscapesAuthority(path) if path.to_string_lossy() == "../escape"
        ));
    }

    #[test]
    fn cross_backend_constructor_stores_source_and_target() {
        assert!(matches!(
            Error::cross_backend_rename(
                PathBuf::parse("/a").unwrap(),
                PathBuf::parse("/b").unwrap(),
            ),
            Error::CrossBackendRename {
                from,
                to,
            } if from.to_string_lossy() == "/a" && to.to_string_lossy() == "/b"
        ));
    }

    #[test]
    fn message_constructor_variants_store_messages() {
        assert!(matches!(
            Error::unsupported("links are disabled"),
            Error::Unsupported {
                message,
            } if message == "links are disabled"
        ));
        assert!(matches!(
            Error::invalid_path("path contains a NUL byte"),
            Error::InvalidPath {
                message,
            } if message == "path contains a NUL byte"
        ));
        assert!(matches!(
            Error::unexpected("host failed", None),
            Error::Unexpected {
                message,
                source: None,
            } if message == "host failed"
        ));
    }
}
