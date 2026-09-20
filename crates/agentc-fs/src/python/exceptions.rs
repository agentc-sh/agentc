// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use agentc_executor_python::{
    backend::ExecutorBackend,
    guestpy::{HostException, host::exception::Raise},
};

use crate::errors::Error;

#[derive(HostException)]
#[guestpy(crate_path = agentc_executor_python::guestpy)]
pub struct FilesystemError {
    #[guestpy(arg)]
    pub(crate) message: String,
}

#[derive(HostException)]
#[guestpy(base = "FilesystemError", crate_path = agentc_executor_python::guestpy)]
pub struct NotFoundError {
    #[guestpy(arg)]
    pub(crate) message: String,
    pub(crate) path: String,
}

#[derive(HostException)]
#[guestpy(base = "FilesystemError", crate_path = agentc_executor_python::guestpy)]
pub struct AlreadyExistsError {
    #[guestpy(arg)]
    pub(crate) message: String,
    pub(crate) path: String,
}

#[derive(HostException)]
#[guestpy(base = "FilesystemError", crate_path = agentc_executor_python::guestpy)]
pub struct NotDirectoryError {
    #[guestpy(arg)]
    pub(crate) message: String,
    pub(crate) path: String,
}

#[derive(HostException)]
#[guestpy(base = "FilesystemError", crate_path = agentc_executor_python::guestpy)]
pub struct IsDirectoryError {
    #[guestpy(arg)]
    pub(crate) message: String,
    pub(crate) path: String,
}

#[derive(HostException)]
#[guestpy(base = "FilesystemError", crate_path = agentc_executor_python::guestpy)]
pub struct PermissionDeniedError {
    #[guestpy(arg)]
    pub(crate) message: String,
    pub(crate) path: String,
}

#[derive(HostException)]
#[guestpy(base = "FilesystemError", crate_path = agentc_executor_python::guestpy)]
pub struct PathEscapesAuthorityError {
    #[guestpy(arg)]
    pub(crate) message: String,
    pub(crate) path: String,
}

#[derive(HostException)]
#[guestpy(base = "FilesystemError", crate_path = agentc_executor_python::guestpy)]
pub struct CrossBackendRenameError {
    #[guestpy(arg)]
    pub(crate) message: String,
    pub(crate) source: String,
    pub(crate) destination: String,
}

#[derive(HostException)]
#[guestpy(base = "FilesystemError", crate_path = agentc_executor_python::guestpy)]
pub struct InvalidPathError {
    #[guestpy(arg)]
    pub(crate) message: String,
}

#[derive(HostException)]
#[guestpy(base = "FilesystemError", crate_path = agentc_executor_python::guestpy)]
pub struct UnsupportedError {
    #[guestpy(arg)]
    pub(crate) message: String,
}

#[derive(HostException)]
#[guestpy(base = "FilesystemError", crate_path = agentc_executor_python::guestpy)]
pub struct UnexpectedError {
    #[guestpy(arg)]
    pub(crate) message: String,
}

impl<B: ExecutorBackend> From<Error> for Raise<B> {
    fn from(error: Error) -> Self {
        match error {
            Error::NotFound(ref path) => Raise::host(NotFoundError {
                path: path.to_string_lossy(),
                message: error.to_string(),
            }),
            Error::AlreadyExists(ref path) => Raise::host(AlreadyExistsError {
                path: path.to_string_lossy(),
                message: error.to_string(),
            }),
            Error::NotDirectory(ref path) => Raise::host(NotDirectoryError {
                path: path.to_string_lossy(),
                message: error.to_string(),
            }),
            Error::IsDirectory(ref path) => Raise::host(IsDirectoryError {
                path: path.to_string_lossy(),
                message: error.to_string(),
            }),
            Error::PermissionDenied(ref path) => Raise::host(PermissionDeniedError {
                path: path.to_string_lossy(),
                message: error.to_string(),
            }),
            Error::Unsupported { .. } => {
                Raise::host(UnsupportedError { message: error.to_string() })
            }
            Error::InvalidPath { .. } => {
                Raise::host(InvalidPathError { message: error.to_string() })
            }
            Error::PathEscapesAuthority(ref path) => Raise::host(PathEscapesAuthorityError {
                path: path.to_string_lossy(),
                message: error.to_string(),
            }),
            Error::CrossBackendRename { ref from, ref to } => {
                Raise::host(CrossBackendRenameError {
                    source: from.to_string_lossy(),
                    destination: to.to_string_lossy(),
                    message: error.to_string(),
                })
            }
            Error::Unexpected { .. } => Raise::host(UnexpectedError { message: error.to_string() }),
        }
    }
}
