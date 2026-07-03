// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use agentc_agent::{
    graph::state::GraphState,
    tools::{
        errors::ToolError,
        traits::TypedTool,
        types::{TypedToolInput, TypedToolOutput},
    },
    types::capability::CapabilitySet,
};

use crate::registry::SkillRegistry;

/// A tool for listing all available skills with their names and descriptions.
pub struct ListSkillsTool {
    pub registry: Arc<SkillRegistry>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ListSkillsInput {}

#[async_trait]
impl<S: GraphState + 'static> TypedTool<S> for ListSkillsTool {
    type Input = ListSkillsInput;
    type Output = String;
    type State = ();
    type StateUpdate = ();

    fn name(&self) -> &str {
        "list_skills"
    }

    fn description(&self) -> &str {
        r#"List all available skills with their names and descriptions.
        Call this to discover what skills are available before deciding
        whether to load one."#
    }

    fn capabilities(&self) -> CapabilitySet {
        CapabilitySet::empty()
    }

    async fn execute(
        &self,
        _input: TypedToolInput<ListSkillsInput>,
    ) -> Result<TypedToolOutput<String, ()>, ToolError> {
        Ok(TypedToolOutput::ok(format!(
            "<skill_catalog>\n{}\n</skill_catalog>",
            self.registry
                .all()
                .map(|s| format!(
                    "  - <name>{}</name>: <description>{}</description>",
                    s.name, s.description
                ))
                .collect::<Vec<_>>()
                .join("\n")
        )))
    }
}
