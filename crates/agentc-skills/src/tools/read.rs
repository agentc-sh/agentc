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

/// A tool for reading any file bundled with a skill.
pub struct ReadSkillFileTool {
    pub registry: Arc<SkillRegistry>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ReadSkillFileInput {
    /// Name of the skill containing the file.
    pub skill_name: String,
    /// Relative path to the file within the skill, e.g. `references/REFERENCE.md`.
    pub file_path: String,
}

#[async_trait]
impl<S: GraphState + 'static> TypedTool<S> for ReadSkillFileTool {
    type Input = ReadSkillFileInput;
    type Output = String;
    type State = ();
    type StateUpdate = ();

    fn name(&self) -> &str {
        "read_skill_file"
    }

    fn description(&self) -> &str {
        r#"Read the contents of any file bundled with a skill, such as reference
        documentation, templates, or assets. Use get_skill first to see which
        files are available."#
    }

    fn capabilities(&self) -> CapabilitySet {
        CapabilitySet::empty()
    }

    async fn execute(
        &self,
        input: TypedToolInput<ReadSkillFileInput>,
    ) -> Result<TypedToolOutput<String, ()>, ToolError> {
        Ok(TypedToolOutput::ok(
            self.registry
                .get(&input.args.skill_name)
                .ok_or_else(|| {
                    ToolError::not_found(format!("skill '{}' not found", input.args.skill_name))
                })?
                .read_resource(&input.args.file_path)
                .await
                .map_err(|e| ToolError::execution_error("read_skill_file", e.to_string()))?,
        ))
    }
}
