// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use agentc_executor_typescript::executor::ExecutorBuilder;

use crate::javascript::bindings::library::ToolsLibrary;

pub trait ExecutorBuilderToolsExt {
    fn with_tools(self) -> Self;
}

impl ExecutorBuilderToolsExt for ExecutorBuilder {
    fn with_tools(self) -> Self {
        self.configure(|runtime| runtime.bind(ToolsLibrary::bind()))
    }
}
