// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::sync::Arc;

use crate::{
    backend::Backend,
    errors::Error,
    fs::filesystem::Fs,
    mount::{Mount, MountFs, MountKind},
    path::{IntoPathBuf, PathBuf},
    policy::{Policy, PolicyFs},
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

    fn mount_path(path: impl IntoPathBuf) -> Result<PathBuf, Error> {
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

    pub fn policy(mut self, policy: impl Policy) -> Self {
        self.policies.push(Arc::new(policy));
        self
    }

    pub fn build(self) -> Result<Fs, Error> {
        if let Some(error) = self.error {
            return Err(error);
        }

        if self.policies.is_empty() {
            return Ok(Fs::new(MountFs::new(self.mounts)));
        }

        Ok(Fs::new(PolicyFs::from_policies(MountFs::new(self.mounts), self.policies)))
    }
}

impl Default for FsBuilder {
    fn default() -> Self {
        FsBuilder::new()
    }
}
