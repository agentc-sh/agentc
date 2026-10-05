// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

use crate::protocol::ids::ToolCallId;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct FunctionCall {
    pub name: String,
    // TODO: More suitable to use JsonValue here?
    pub arguments: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ToolCall {
    pub id: ToolCallId,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: FunctionCall,
}

impl ToolCall {
    pub fn new(id: impl Into<ToolCallId>, function: FunctionCall) -> Self {
        Self {
            id: id.into(),
            call_type: "function".to_string(),
            function,
        }
    }
}

/// A tool definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct Tool {
    /// The tool name
    pub name: String,
    /// The tool description
    pub description: String,
    /// The tool parameters
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Value>,
}

impl Tool {
    pub fn new(name: String, description: String, parameters: Option<Value>) -> Self {
        Self { name, description, parameters }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{from_value, json, to_value};

    use crate::protocol::tool::Tool;

    #[test]
    fn parameters_are_optional() {
        let tool = from_value::<Tool>(json!({
            "name": "search",
            "description": "Search"
        }))
        .unwrap();

        assert_eq!(tool.parameters, None);
        assert_eq!(
            to_value(tool).unwrap(),
            json!({ "name": "search", "description": "Search" })
        );
    }
}
