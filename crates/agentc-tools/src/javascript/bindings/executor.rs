// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use agentc_executor_typescript::{backend::ExecutorBackend, executor::ExecutorBuilder};

use crate::javascript::bindings::library::ToolsLibrary;

pub trait ExecutorBuilderToolsExt {
    fn with_tools(self) -> Self;
}

impl<B: ExecutorBackend> ExecutorBuilderToolsExt for ExecutorBuilder<B> {
    fn with_tools(self) -> Self {
        self.configure(|runtime| Ok(runtime.bind(ToolsLibrary::bind::<B>()?)))
    }
}
