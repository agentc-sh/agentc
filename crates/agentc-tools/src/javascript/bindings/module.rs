// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::borrow::Cow;

use agentc_executor_typescript::guestjs::host::{Exports, HostModule};

use crate::javascript::bindings::{input::ToolInput, schema::Schema, tool::Tool};

pub struct ToolsModule {
    specifier: Cow<'static, str>,
}

impl ToolsModule {
    const DEFAULT_SPECIFIER: &'static str = "agentc:tools";

    pub fn new() -> Self {
        Self {
            specifier: Cow::Borrowed(Self::DEFAULT_SPECIFIER),
        }
    }

    pub fn with_specifier(mut self, specifier: impl Into<Cow<'static, str>>) -> Self {
        self.specifier = specifier.into();
        self
    }

    pub fn specifier(&self) -> &str {
        &self.specifier
    }
}

impl Default for ToolsModule {
    fn default() -> Self {
        Self::new()
    }
}

impl HostModule for ToolsModule {
    fn name(&self) -> &str {
        self.specifier()
    }

    fn build(&self, exports: &mut Exports) {
        exports.class::<Tool>();
        exports.class::<ToolInput>();
        exports.class::<Schema>();
    }
}
