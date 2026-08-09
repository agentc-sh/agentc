// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::sync::Arc;

use crate::{
    backend::{Backend, ErasedBackend},
    fs::{builder::FsBuilder, dir::Dir},
    memory::MemoryFs,
    path::PathBuf,
};

#[derive(Clone)]
pub struct Fs {
    pub(crate) backend: Arc<dyn ErasedBackend>,
}

impl Fs {
    pub fn builder() -> FsBuilder {
        FsBuilder::new()
    }

    pub(crate) fn new_erased(backend: Arc<dyn ErasedBackend>) -> Self {
        Fs { backend }
    }

    pub fn new(backend: impl Backend) -> Self {
        Fs::new_erased(Arc::new(backend))
    }

    pub fn memory() -> Self {
        Fs::new(MemoryFs::new())
    }

    pub fn root(&self) -> Dir {
        Dir::new(self.clone(), PathBuf::root(), PathBuf::root())
    }
}
