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

/// A tool for loading the full instructions of a skill by name.
pub struct GetSkillTool {
    pub registry: Arc<SkillRegistry>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GetSkillInput {
    /// The name of the skill to load.
    pub name: String,
}

#[async_trait]
impl<S: GraphState + 'static> TypedTool<S> for GetSkillTool {
    type Input = GetSkillInput;
    type Output = String;
    type State = ();
    type StateUpdate = ();

    fn name(&self) -> &str {
        "get_skill"
    }

    fn description(&self) -> &str {
        r#"Load the full instructions for a skill by name. Returns the skill body,
        its directory path (if available), and a listing of any bundled resource
        files. Use list_skills first to discover available skill names."#
    }

    fn capabilities(&self) -> CapabilitySet {
        CapabilitySet::empty()
    }

    async fn execute(
        &self,
        input: TypedToolInput<GetSkillInput>,
    ) -> Result<TypedToolOutput<String, ()>, ToolError> {
        let skill = self
            .registry
            .get(&input.args.name)
            .ok_or_else(|| {
                ToolError::not_found(format!("skill '{}' not found", input.args.name))
            })?;

        let mut parts = vec![skill.body.clone()];

        if let Some(base_dir) = &skill.base_dir {
            parts.push(format!("\nSkill directory: {}", base_dir.display()));
        }

        if !skill.resources.is_empty() {
            parts.push(format!(
                "\n<skill_resources>\n{}\n</skill_resources>",
                skill
                    .resources
                    .iter()
                    .map(|r| format!("  <file>{}</file>", r))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }

        Ok(TypedToolOutput::ok(format!(
            "<skill_content name=\"{}\">\n{}\n</skill_content>",
            skill.name,
            parts.join("\n"),
        )))
    }
}
