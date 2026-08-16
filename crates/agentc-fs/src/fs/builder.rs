// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::sync::Arc;

use crate::{
    backend::Backend,
    errors::Error,
    fs::{
        filesystem::Fs,
        namespace::{Mount, MountKind, Namespace},
    },
    path::{IntoPathBuf, PathBuf},
    policy::Policy,
};

pub struct FsBuilder {
    mounts: Vec<Mount>,
    policies: Vec<Arc<dyn Policy>>,
    error: Option<Error>,
}

impl FsBuilder {
    pub fn new() -> Self {
        FsBuilder {
            mounts: Vec::new(),
            policies: Vec::new(),
            error: None,
        }
    }

    pub(crate) fn mount_path(path: impl IntoPathBuf) -> Result<PathBuf, Error> {
        let path = path.into_path_buf()?.normalize()?;

        if path.is_relative() {
            return Err(Error::invalid_path("mount paths must be absolute"));
        }

        Ok(path)
    }

    pub fn mount(mut self, path: impl IntoPathBuf, backend: impl Backend) -> Self {
        match Self::mount_path(path) {
            Ok(path) => self.mounts.push(Mount {
                path,
                kind: MountKind::Directory,
                backend: Arc::new(backend),
                dev: 0,
            }),
            Err(error) => self.error = Some(error),
        }

        self
    }

    pub fn mount_file(mut self, path: impl IntoPathBuf, backend: impl Backend) -> Self {
        match Self::mount_path(path) {
            Ok(path) => self.mounts.push(Mount {
                path,
                kind: MountKind::File,
                backend: Arc::new(backend),
                dev: 0,
            }),
            Err(error) => self.error = Some(error),
        }

        self
    }

    pub fn mount_fs(mut self, path: impl IntoPathBuf, fs: Fs) -> Self {
        match Self::mount_path(path) {
            Ok(path) => self.mounts.push(Mount {
                path,
                kind: MountKind::Directory,
                backend: fs.namespace,
                dev: 0,
            }),
            Err(error) => self.error = Some(error),
        }

        self
    }

    pub fn policy(mut self, policy: impl Policy) -> Self {
        self.policies.push(Arc::new(policy));
        self
    }

    pub fn build(self) -> Result<Fs, Error> {
        if let Some(error) = self.error {
            return Err(error);
        }

        Ok(Fs::from_namespace(Namespace::new(self.mounts, self.policies)))
    }
}

impl Default for FsBuilder {
    fn default() -> Self {
        FsBuilder::new()
    }
}
