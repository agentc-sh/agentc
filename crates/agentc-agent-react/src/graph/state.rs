// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use json_patch::{AddOperation, Patch, PatchOperation, patch};
use serde::{Deserialize, Serialize};
use serde_json::{Value, to_value};
use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
};
use uuid::Uuid;

use agentc_agent::{
    graph::{
        errors::GraphError,
        state::{FromStateUpdate, GraphState, GraphStateInput, GraphStateUpdate, IntoStateUpdate},
    },
    types::{
        capability::CapabilityOverride,
        tools::{ToolCall, ToolDefinition},
    },
};

use crate::types::{
    context_var::ContextVar,
    message::{AssistantMessage, Message},
    model::ModelConfig,
};

/// The main state corresponding to a specific session of an agent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReActState {
    /// A unique identifier for the run of the agent.
    pub run_id: Uuid,
    /// A unique identifier for the session within the agent.
    pub session_id: Uuid,
    /// Model configuration for this agent session.
    pub model: Option<ModelConfig>,
    /// Override the capabilities for this agent session.
    pub capability_override: Option<CapabilityOverride>,
    /// The messages exchanged in the agent's conversation.
    pub messages: Vec<Message>,
    /// Additional context variables relevant to the agent's operation.
    pub context_vars: Vec<ContextVar>,
    /// The tools available to the agent.
    pub tools: Vec<ToolDefinition>,
    /// Additional arbitrary context that can be used by the agent or tools, not structured as variables.
    pub context: Value,
}

impl ReActState {
    pub fn pending_tool_calls(&self) -> Option<(AssistantMessage, Vec<ToolCall>)> {
        self.messages
            .iter()
            .rposition(|m| m.as_assistant().is_some())
            .and_then(|idx| {
                let assistant = self.messages[idx].as_assistant()?;
                let calls = assistant
                    .tool_calls
                    .iter()
                    .flatten()
                    .filter(|call| {
                        !self.messages[idx + 1..]
                            .iter()
                            .filter_map(Message::as_tool)
                            .any(|tool| tool.tool_call_id == call.id)
                    })
                    .cloned()
                    .collect::<Vec<_>>();

                (!calls.is_empty()).then(|| (assistant.clone(), calls))
            })
    }
}

/// Updates that can be applied to the `ReActState`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReActStateUpdate {
    /// New messages to be added to the agent's conversation.
    pub messages: Vec<Message>,
    /// Patches to the arbitrary context.
    pub context: Vec<PatchOperation>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "serde_with::rust::double_option"
    )]
    pub model: Option<Option<ModelConfig>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "serde_with::rust::double_option"
    )]
    pub capability_override: Option<Option<CapabilityOverride>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_vars: Option<Vec<ContextVar>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolDefinition>>,
}

impl ReActStateUpdate {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            context: Vec::new(),
            model: None,
            capability_override: None,
            context_vars: None,
            tools: None,
        }
    }

    pub fn with_messages<I, M>(mut self, messages: I) -> Self
    where
        I: IntoIterator<Item = M>,
        M: Into<Message>,
    {
        self.messages
            .extend(messages.into_iter().map(Into::into));
        self
    }

    pub fn with_context_patches<I>(mut self, patches: I) -> Self
    where
        I: IntoIterator<Item = PatchOperation>,
    {
        self.context.extend(patches);
        self
    }
}

impl Default for ReActStateUpdate {
    fn default() -> Self {
        Self::new()
    }
}

impl FromStateUpdate<ReActStateUpdate> for Patch {
    fn from_update(update: ReActStateUpdate) -> Result<Option<Self>, GraphError> {
        let mut operations = Vec::new();

        if !update.messages.is_empty() {
            operations.push(PatchOperation::Add(AddOperation {
                path: "/messages"
                    .try_into()
                    .map_err(GraphError::conversion_error)?,
                value: to_value(update.messages).map_err(GraphError::conversion_error)?,
            }));
        }

        if let Some(model) = update.model {
            operations.push(PatchOperation::Add(AddOperation {
                path: "/model"
                    .try_into()
                    .map_err(GraphError::conversion_error)?,
                value: to_value(model).map_err(GraphError::conversion_error)?,
            }));
        }

        if let Some(capability_override) = update.capability_override {
            operations.push(PatchOperation::Add(AddOperation {
                path: "/capability_override"
                    .try_into()
                    .map_err(GraphError::conversion_error)?,
                value: to_value(capability_override).map_err(GraphError::conversion_error)?,
            }));
        }

        if let Some(context_vars) = update.context_vars {
            operations.push(PatchOperation::Add(AddOperation {
                path: "/context_vars"
                    .try_into()
                    .map_err(GraphError::conversion_error)?,
                value: to_value(context_vars).map_err(GraphError::conversion_error)?,
            }));
        }

        if let Some(tools) = update.tools {
            operations.push(PatchOperation::Add(AddOperation {
                path: "/tools"
                    .try_into()
                    .map_err(GraphError::conversion_error)?,
                value: to_value(tools).map_err(GraphError::conversion_error)?,
            }));
        }

        operations.extend(
            update
                .context
                .into_iter()
                // Ensure all context patch operations are prefixed with "/context"
                .filter_map(|patch_op| match patch_op {
                    PatchOperation::Add(mut add_op) => {
                        add_op.path = format!("/context{}", add_op.path)
                            .try_into()
                            .ok()?;
                        Some(PatchOperation::Add(add_op))
                    }
                    PatchOperation::Remove(mut remove_op) => {
                        remove_op.path = format!("/context{}", remove_op.path)
                            .try_into()
                            .ok()?;
                        Some(PatchOperation::Remove(remove_op))
                    }
                    PatchOperation::Replace(mut replace_op) => {
                        replace_op.path = format!("/context{}", replace_op.path)
                            .try_into()
                            .ok()?;
                        Some(PatchOperation::Replace(replace_op))
                    }
                    PatchOperation::Move(mut move_op) => {
                        move_op.from = format!("/context{}", move_op.from)
                            .try_into()
                            .ok()?;
                        move_op.path = format!("/context{}", move_op.path)
                            .try_into()
                            .ok()?;
                        Some(PatchOperation::Move(move_op))
                    }
                    PatchOperation::Copy(mut copy_op) => {
                        copy_op.from = format!("/context{}", copy_op.from)
                            .try_into()
                            .ok()?;
                        copy_op.path = format!("/context{}", copy_op.path)
                            .try_into()
                            .ok()?;
                        Some(PatchOperation::Copy(copy_op))
                    }
                    PatchOperation::Test(mut test_op) => {
                        test_op.path = format!("/context{}", test_op.path)
                            .try_into()
                            .ok()?;
                        Some(PatchOperation::Test(test_op))
                    }
                }),
        );

        Ok(Some(Patch(operations)))
    }
}

/// Input required to initialize the `ReActState`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReActStateInput {
    /// A unique identifier for the run of the agent.
    pub run_id: Uuid,
    /// A unique identifier for the session within the agent.
    pub session_id: Uuid,
    /// Model configuration for this agent session.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "serde_with::rust::double_option"
    )]
    pub model: Option<Option<ModelConfig>>,
    /// Override the capabilities for this agent session.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "serde_with::rust::double_option"
    )]
    pub capability_override: Option<Option<CapabilityOverride>>,
    /// New messages for the agent's conversation.
    pub messages: Vec<Message>,
    /// Initial context variables for the agent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_vars: Option<Vec<ContextVar>>,
    /// The tools available to the agent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolDefinition>>,
    /// Additional arbitrary context that can be used by the agent or tools, not structured as variables.
    pub context: Value,
}

impl Default for ReActStateInput {
    fn default() -> Self {
        Self {
            model: None,
            capability_override: None,
            run_id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            messages: Vec::new(),
            context_vars: None,
            tools: None,
            context: Value::Object(Default::default()),
        }
    }
}

impl IntoStateUpdate<ReActStateUpdate> for ReActStateInput {
    fn into_update(self) -> Result<Option<ReActStateUpdate>, GraphError> {
        Ok(
            Some(
                ReActStateUpdate {
                    messages: self.messages,
                    // Convert the context Value into RFC 6902 `add` operations, one per
                    // top-level key. `add` replaces an existing key or inserts a missing
                    // one, giving shallow-merge semantics without needing the current state.
                    // Non-object values produce no operations.
                    context: match self.context {
                        Value::Object(map) => map
                            .into_iter()
                            .filter_map(|(key, value)| {
                                format!(
                                    "/{}",
                                    key.replace('~', "~0")
                                        .replace('/', "~1")
                                )
                                .try_into()
                                .ok()
                                .map(|path| PatchOperation::Add(AddOperation { path, value }))
                            })
                            .collect(),
                        _ => Vec::new(),
                    },
                    model: self.model,
                    capability_override: self.capability_override,
                    context_vars: self.context_vars,
                    tools: self.tools,
                }
            )
        )
    }
}

impl GraphState for ReActState {
    type Update = ReActStateUpdate;
    type Input = ReActStateInput;
}

impl GraphStateUpdate for ReActStateUpdate {
    type State = ReActState;

    fn apply(self, state: &mut Self::State) {
        // Deduplicate messages by ID and backfill tool message
        // parent message IDs if missing
        let seen = state
            .messages
            .iter()
            .map(|message| *message.id())
            .collect::<HashSet<_>>();

        let tool_call_map = state
            .messages
            .iter()
            .filter_map(|message| message.as_assistant())
            .filter_map(|message| {
                message
                    .tool_calls
                    .as_ref()
                    .map(|tool_calls| (*message.id(), tool_calls.clone()))
            })
            .flat_map(|(message_id, tool_calls)| {
                tool_calls
                    .into_iter()
                    .map(move |tool_call| (tool_call.id, message_id))
            })
            .collect::<HashMap<_, _>>();

        state.messages.extend(
            self.messages
                .into_iter()
                .filter(|message| !seen.contains(message.id()))
                .map(|message| match message {
                    Message::Tool(mut tool_message) if tool_message.parent_message_id.is_none() => {
                        if let Some(parent_message_id) =
                            tool_call_map.get(&tool_message.tool_call_id)
                        {
                            tool_message.parent_message_id = Some(*parent_message_id);
                        }

                        Message::Tool(tool_message)
                    }
                    other => other,
                }),
        );

        if !self.context.is_empty() {
            let _ = patch(&mut state.context, &self.context);
        }

        if let Some(model) = self.model {
            state.model = model;
        }

        if let Some(value) = self.capability_override {
            state.capability_override = value;
        }

        if let Some(vars) = self.context_vars {
            state.context_vars = vars;
        }

        if let Some(tools) = self.tools {
            state.tools = tools;
        }
    }

    fn merge(mut self, other: Self) -> Self {
        self.messages.extend(other.messages);
        self.context.extend(other.context);

        if other.model.is_some() {
            self.model = other.model;
        }

        if other.capability_override.is_some() {
            self.capability_override = other.capability_override;
        }

        if other.context_vars.is_some() {
            self.context_vars = other.context_vars;
        }

        if other.tools.is_some() {
            self.tools = other.tools;
        }

        self
    }
}

impl GraphStateInput for ReActStateInput {
    type State = ReActState;

    fn initialize(self) -> Self::State {
        ReActState {
            model: self.model.flatten(),
            capability_override: self.capability_override.flatten(),
            run_id: self.run_id,
            session_id: self.session_id,
            messages: self.messages,
            context_vars: self.context_vars.unwrap_or_default(),
            tools: self.tools.unwrap_or_default(),
            context: self.context,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agentc_agent::graph::state::GraphStateUpdate;
    use json_patch::{AddOperation, PatchOperation};
    use serde_json::json;

    fn base_state() -> ReActState {
        ReActStateInput::default().initialize()
    }

    #[test]
    fn context_patch_applied_on_apply() {
        let mut state = base_state();

        ReActStateUpdate {
            messages: vec![],
            context: vec![PatchOperation::Add(AddOperation {
                path: "/foo".try_into().unwrap(),
                value: json!("bar"),
            })],
            ..Default::default()
        }
        .apply(&mut state);

        assert_eq!(state.context["foo"], json!("bar"));
    }

    #[test]
    fn context_patches_merged_in_order() {
        let mut state = base_state();

        ReActStateUpdate {
            messages: vec![],
            context: vec![PatchOperation::Add(AddOperation {
                path: "/a".try_into().unwrap(),
                value: json!(1),
            })],
            ..Default::default()
        }
        .merge(ReActStateUpdate {
            messages: vec![],
            context: vec![PatchOperation::Add(AddOperation {
                path: "/b".try_into().unwrap(),
                value: json!(2),
            })],
            ..Default::default()
        })
        .apply(&mut state);

        assert_eq!(state.context["a"], json!(1));
        assert_eq!(state.context["b"], json!(2));
    }

    #[test]
    fn state_updates_preserve_client_model_config() {
        let model = ModelConfig::new().with_timeout(250);
        let mut state = ReActStateInput {
            model: Some(Some(model.clone())),
            ..Default::default()
        }
        .initialize();

        ReActStateUpdate::new()
            .with_context_patches([PatchOperation::Add(AddOperation {
                path: "/updated".try_into().unwrap(),
                value: json!(true),
            })])
            .apply(&mut state);

        assert_eq!(state.model, Some(model));
    }

    #[test]
    fn run_input_preserves_omitted_values_and_clears_explicit_values() {
        let model = ModelConfig::new().with_timeout(250);
        let capability = CapabilityOverride::Inherit;
        let context_var = ContextVar {
            description: "prior".into(),
            value: "value".into(),
        };
        let tool = ToolDefinition {
            name: "prior_tool".into(),
            description: "prior".into(),
            parameters: json!({}),
        };

        let mut state = ReActStateInput {
            model: Some(Some(model.clone())),
            capability_override: Some(Some(capability.clone())),
            context_vars: Some(vec![context_var.clone()]),
            tools: Some(vec![tool.clone()]),
            ..Default::default()
        }
        .initialize();

        ReActStateInput::default()
            .into_update()
            .unwrap()
            .unwrap()
            .apply(&mut state);

        assert_eq!(state.model, Some(model));
        assert_eq!(state.capability_override, Some(capability));
        assert_eq!(state.context_vars, vec![context_var]);
        assert_eq!(state.tools, vec![tool]);

        ReActStateInput {
            model: Some(None),
            capability_override: Some(None),
            context_vars: Some(Vec::new()),
            tools: Some(Vec::new()),
            ..Default::default()
        }
        .into_update()
        .unwrap()
        .unwrap()
        .apply(&mut state);

        assert_eq!(state.model, None);
        assert_eq!(state.capability_override, None);
        assert!(state.context_vars.is_empty());
        assert!(state.tools.is_empty());
    }

    #[test]
    fn later_specified_update_wins_without_losing_earlier_values() {
        let model = ModelConfig::new().with_timeout(250);
        let tool = ToolDefinition {
            name: "prior_tool".into(),
            description: "prior".into(),
            parameters: json!({}),
        };

        let merged = ReActStateUpdate {
            model: Some(Some(model.clone())),
            capability_override: Some(Some(CapabilityOverride::Inherit)),
            context_vars: Some(vec![]),
            tools: Some(vec![tool]),
            ..Default::default()
        }
        .merge(ReActStateUpdate {
            model: Some(None),
            tools: Some(vec![]),
            ..Default::default()
        });

        assert_eq!(merged.model, Some(None));
        assert_eq!(
            merged.capability_override,
            Some(Some(CapabilityOverride::Inherit))
        );
        assert_eq!(merged.context_vars, Some(vec![]));
        assert_eq!(merged.tools, Some(vec![]));

        let unchanged = merged.merge(ReActStateUpdate::default());

        assert_eq!(unchanged.model, Some(None));
        assert_eq!(unchanged.tools, Some(vec![]));
    }

    #[test]
    fn update_serialization_preserves_omission_and_explicit_clear() {
        let default = serde_json::to_value(ReActStateUpdate::default()).unwrap();

        assert!(default.get("model").is_none());
        assert!(default.get("capability_override").is_none());
        assert!(default.get("context_vars").is_none());
        assert!(default.get("tools").is_none());

        let clear = serde_json::from_value::<ReActStateUpdate>(
            json!({
                "messages": [],
                "context": [],
                "model": null,
                "capability_override": null,
                "context_vars": [],
                "tools": []
            }),
        )
        .unwrap();

        assert_eq!(clear.model, Some(None));
        assert_eq!(clear.capability_override, Some(None));
        assert_eq!(clear.context_vars, Some(vec![]));
        assert_eq!(clear.tools, Some(vec![]));
        assert_eq!(
            serde_json::to_value(clear).unwrap(),
            json!({
                "messages": [],
                "context": [],
                "model": null,
                "capability_override": null,
                "context_vars": [],
                "tools": []
            })
        );
    }

    #[test]
    fn generic_patch_conversion_preserves_override_values() {
        let clear = ReActStateUpdate {
            model: Some(None),
            capability_override: Some(None),
            context_vars: Some(vec![]),
            tools: Some(vec![]),
            ..Default::default()
        };
        let patch = Patch::from_update(clear.clone()).unwrap().unwrap();

        assert_eq!(
            serde_json::to_value(&patch).unwrap(),
            json!([
                {"op": "add", "path": "/model", "value": null},
                {"op": "add", "path": "/capability_override", "value": null},
                {"op": "add", "path": "/context_vars", "value": []},
                {"op": "add", "path": "/tools", "value": []}
            ])
        );
        assert_eq!(
            <Patch as IntoStateUpdate<ReActStateUpdate>>::into_update(patch)
                .unwrap()
                .unwrap(),
            clear
        );
    }
}
