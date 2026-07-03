// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
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

/// A tool for describing a skill's frontmatter metadata without loading its full instructions.
pub struct DescribeSkillTool {
    pub registry: Arc<SkillRegistry>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct DescribeSkillInput {
    /// The name of the skill to describe.
    pub name: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct DescribeSkillOutput {
    name: String,
    description: String,
    #[serde(flatten)]
    extra: Value,
}

#[async_trait]
impl<S: GraphState + 'static> TypedTool<S> for DescribeSkillTool {
    type Input = DescribeSkillInput;
    type Output = DescribeSkillOutput;
    type State = ();
    type StateUpdate = ();

    fn name(&self) -> &str {
        "describe_skill"
    }

    fn description(&self) -> &str {
        r#"Return the frontmatter metadata for a skill without loading its full
        instructions. Useful for inspecting compatibility or other metadata
        fields before deciding whether to activate the skill."#
    }

    fn capabilities(&self) -> CapabilitySet {
        CapabilitySet::empty()
    }

    async fn execute(
        &self,
        input: TypedToolInput<DescribeSkillInput>,
    ) -> Result<TypedToolOutput<DescribeSkillOutput, ()>, ToolError> {
        let skill = self
            .registry
            .get(&input.args.name)
            .ok_or_else(|| {
                ToolError::not_found(format!("skill '{}' not found", input.args.name))
            })?;

        Ok(TypedToolOutput::ok(DescribeSkillOutput {
            name: skill.name.clone(),
            description: skill.description.clone(),
            extra: skill.extra_frontmatter.clone(),
        }))
    }
}
