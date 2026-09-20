// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use agentc_executor_python::{backend::ExecutorBackend, executor::ExecutorBuilder};

use crate::{fs::Dir, python::library::FsLibrary};

pub trait ExecutorBuilderFsExt {
    fn with_fs(self, dir: Dir) -> Self;
}

impl<B: ExecutorBackend> ExecutorBuilderFsExt for ExecutorBuilder<B> {
    fn with_fs(self, dir: Dir) -> Self {
        self.configure(move |runtime| Ok(runtime.bind(FsLibrary::bind::<B>(dir.clone())?)))
    }
}
