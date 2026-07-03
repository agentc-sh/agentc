// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};
use serde_json::Value;

use agentc_model::types::tools::{ToolCall as ModelToolCall, ToolSpec as ModelToolSpec};

use crate::types::conversion::{FromModelType, ToModelType};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: Value,
}

impl ToModelType for ToolDefinition {
    type ModelType = ModelToolSpec;

    fn to_model_type(&self) -> Self::ModelType {
        ModelToolSpec {
            name: self.name.clone(),
            description: self.description.clone(),
            parameters: self.parameters.clone(),
        }
    }
}

impl FromModelType for ToolCall {
    type ModelType = ModelToolCall;
    type Output = Self;

    fn from_model_type(model: Self::ModelType) -> Self::Output {
        Self {
            id: model.id,
            name: model.name,
            arguments: model.arguments,
        }
    }
}

impl ToModelType for ToolCall {
    type ModelType = ModelToolCall;

    fn to_model_type(&self) -> Self::ModelType {
        ModelToolCall {
            id: self.id.clone(),
            name: self.name.clone(),
            arguments: self.arguments.clone(),
        }
    }
}
