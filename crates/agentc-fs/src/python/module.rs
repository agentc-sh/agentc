// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::borrow::Cow;

use agentc_executor_python::{
    backend::ExecutorBackend,
    guestpy::{errors::Error, host::module::ModuleSpec},
};

use crate::{
    fs::Dir,
    python::{
        directory::Directory,
        entry::Entry,
        exceptions::{
            AlreadyExistsError, CrossBackendRenameError, FilesystemError, InvalidPathError,
            IsDirectoryError, NotDirectoryError, NotFoundError, PathEscapesAuthorityError,
            PermissionDeniedError, UnexpectedError, UnsupportedError,
        },
        file::File,
        stat::Stat,
    },
};

pub struct FsModule {
    name: Cow<'static, str>,
    dir: Dir,
}

impl FsModule {
    const DEFAULT_NAME: &'static str = "agentc_fs";

    pub fn new(dir: Dir) -> Self {
        Self {
            name: Cow::Borrowed(Self::DEFAULT_NAME),
            dir,
        }
    }

    pub fn with_name(mut self, name: impl Into<Cow<'static, str>>) -> Self {
        self.name = name.into();
        self
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn dir(&self) -> &Dir {
        &self.dir
    }
}

impl<B: ExecutorBackend> TryFrom<FsModule> for ModuleSpec<B> {
    type Error = Error;

    fn try_from(module: FsModule) -> Result<Self, Self::Error> {
        Ok(
            ModuleSpec::new(module.name.clone())
                .state(module)
                .exception_type::<FilesystemError>()
                .exception_type::<NotFoundError>()
                .exception_type::<AlreadyExistsError>()
                .exception_type::<NotDirectoryError>()
                .exception_type::<IsDirectoryError>()
                .exception_type::<PermissionDeniedError>()
                .exception_type::<PathEscapesAuthorityError>()
                .exception_type::<CrossBackendRenameError>()
                .exception_type::<InvalidPathError>()
                .exception_type::<UnsupportedError>()
                .exception_type::<UnexpectedError>()
                .class::<Stat<B>>()?
                .class::<Entry<B>>()?
                .class::<File<B>>()?
                .class::<Directory<B>>()?
        )
    }
}
