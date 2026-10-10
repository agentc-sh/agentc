// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::borrow::Cow;

use agentc_executor_python::guestpy::{
    backend::{
        Backend, BackendCallables, BackendClasses, BackendCoroutines, BackendExceptions,
        BackendModules, BackendValues,
    },
    errors::Error,
    host::module::ModuleSpec,
};

use crate::python::bindings::{input::ToolInput, output::ToolOutput, schema::Schema, tool::Tool};

pub struct ToolsModule {
    name: Cow<'static, str>,
}

impl ToolsModule {
    const DEFAULT_NAME: &'static str = "agentc_tools";

    pub fn new() -> Self {
        Self { name: Cow::Borrowed(Self::DEFAULT_NAME) }
    }

    pub fn with_name(mut self, name: impl Into<Cow<'static, str>>) -> Self {
        self.name = name.into();
        self
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

impl Default for ToolsModule {
    fn default() -> Self {
        Self::new()
    }
}

impl<B> TryFrom<ToolsModule> for ModuleSpec<B>
where
    B: Backend
        + BackendValues
        + BackendCallables
        + BackendClasses
        + BackendModules
        + BackendCoroutines
        + BackendExceptions,
{
    type Error = Error;

    fn try_from(module: ToolsModule) -> Result<Self, Self::Error> {
        Ok(ModuleSpec::new(module.name)
            .class::<Tool>()?
            .class::<ToolInput<B>>()?
            .class::<ToolOutput<B>>()?
            .class::<Schema>()?)
    }
}
