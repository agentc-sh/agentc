// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use agentc_agent::types::capability::{Capability, CapabilitySet};
use agentc_executor_typescript::{executor::ExecutorBuilder, guestjs::llrt::Llrt};

/// Extends a TypeScript executor builder with JavaScript tool capabilities.
pub trait ExecutorBuilderToolExt {
    /// Adds guest facilities granted by a JavaScript package's tool capabilities.
    fn with_tool_capabilities<I>(self, capabilities: I) -> Self
    where
        I: Into<CapabilitySet>;
}

impl ExecutorBuilderToolExt for ExecutorBuilder {
    fn with_tool_capabilities<I>(self, capabilities: I) -> Self
    where
        I: Into<CapabilitySet>,
    {
        let filesystem = capabilities
            .into()
            .has_any(&[
                Capability::from("filesystem::read"),
                Capability::from("filesystem::write"),
            ]);

        self.configure(move |builder| {
            builder.bind_native(match filesystem {
                true => Llrt::builder().fs().build(),
                false => Llrt::builder().build(),
            })
        })
    }
}
