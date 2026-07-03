// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use rig::completion::ToolDefinition;

use crate::{errors::ModelError, types::tools::ToolSpec};

impl TryFrom<ToolSpec> for ToolDefinition {
    type Error = ModelError;

    fn try_from(value: ToolSpec) -> Result<Self, Self::Error> {
        Ok(Self {
            name: value.name,
            description: value.description,
            parameters: value.parameters,
        })
    }
}
