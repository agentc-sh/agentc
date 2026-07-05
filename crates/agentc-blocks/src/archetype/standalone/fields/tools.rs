// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use crate::{
    archetype::standalone::fields::spec::{FieldsSpec, IntoFieldSpecs},
    context::{ResolvedContextTool, ResolvedContextToolKind},
};

/// Pairs a tool name with its resolved context for use with [`IntoFieldSpecs`].
///
/// Tool field paths include the tool name (e.g. `["tool", "my_tool", "enabled"]`),
/// so the name must accompany the tool when building field specs.
pub struct NamedTool<'a>(pub &'a str, pub &'a ResolvedContextTool);

impl IntoFieldSpecs for NamedTool<'_> {
    fn extend_fields(&self, fields: &mut FieldsSpec) {
        let (name, tool) = (self.0, self.1);

        match &tool.kind {
            ResolvedContextToolKind::Javascript(_) | ResolvedContextToolKind::Python(_) => {
                fields.push(&["tool", name, "enabled"], &tool.enabled);

                for (config_key, config_value) in &tool.config {
                    fields.push(&["tool", name, config_key.as_str()], config_value);
                }
            }

            // MCP loader calls are contributed to `config::loader` by AgentCodeGen.
            ResolvedContextToolKind::Mcp(_) => {}

            // Bash tools have no runtime-configurable fields beyond what is baked at compile time.
            ResolvedContextToolKind::Bash(_) => {}
        }
    }
}
