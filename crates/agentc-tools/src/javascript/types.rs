// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use agentc_agent::{tools::activity::ActivityDelta, types::tools::ToolDefinition};
use agentc_executor_typescript::guestjs::{FromGuest, ToGuest};

#[derive(serde::Deserialize, FromGuest)]
#[guestjs(crate_path = agentc_executor_typescript::guestjs)]
pub(crate) struct JavascriptToolDefinition {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

impl From<JavascriptToolDefinition> for ToolDefinition {
    fn from(definition: JavascriptToolDefinition) -> Self {
        Self {
            name: definition.name,
            description: definition.description,
            parameters: definition.parameters,
        }
    }
}

#[derive(serde::Deserialize, FromGuest)]
#[guestjs(crate_path = agentc_executor_typescript::guestjs)]
pub(crate) struct JavascriptToolResult {
    pub(crate) output: serde_json::Value,
    pub(crate) state_update: Option<json_patch::Patch>,
}

#[derive(Clone, serde::Serialize, ToGuest)]
#[guestjs(crate_path = agentc_executor_typescript::guestjs)]
#[serde(transparent)]
pub(crate) struct JavascriptValue(serde_json::Value);

impl JavascriptValue {
    pub(crate) fn new(value: serde_json::Value) -> Self {
        Self(value)
    }
}

#[derive(serde::Deserialize, FromGuest)]
#[guestjs(crate_path = agentc_executor_typescript::guestjs)]
#[serde(transparent)]
pub(crate) struct JavascriptActivityDelta(ActivityDelta);

impl From<JavascriptActivityDelta> for ActivityDelta {
    fn from(delta: JavascriptActivityDelta) -> Self {
        delta.0
    }
}
