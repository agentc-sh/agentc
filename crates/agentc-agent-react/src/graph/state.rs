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
    types::{capability::CapabilityOverride, tools::ToolDefinition},
};

use crate::types::{context_var::ContextVar, message::Message, model::ModelOverride};

/// The main state corresponding to a specific session of an agent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReActState {
    /// A unique identifier for the run of the agent.
    pub run_id: Uuid,
    /// A unique identifier for the session within the agent.
    pub session_id: Uuid,
    /// Override the model to be used for this agent session.
    pub model_override: Option<ModelOverride>,
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

/// Updates that can be applied to the `ReActState`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReActStateUpdate {
    /// New messages to be added to the agent's conversation.
    pub messages: Vec<Message>,
    /// Patches to the arbitrary context.
    pub context: Vec<PatchOperation>,
}

impl ReActStateUpdate {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            context: Vec::new(),
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
        // Manually add the messages patch, then unpack all of the context patches from the update
        // and mutate all of their paths to start with /context so that they apply to the context field of the state.
        Ok(Some(Patch(
            (!update.messages.is_empty())
                .then(|| {
                    Some(PatchOperation::Add(AddOperation {
                        path: "/messages".try_into().ok()?,
                        value: to_value(update.messages).expect("failed to serialize messages"),
                    }))
                })
                .flatten()
                .into_iter()
                .chain(
                    update
                        .context
                        .into_iter()
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
                )
                .collect(),
        )))
    }
}

/// Input required to initialize the `ReActState`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReActStateInput {
    /// A unique identifier for the run of the agent.
    pub run_id: Uuid,
    /// A unique identifier for the session within the agent.
    pub session_id: Uuid,
    /// Override the model to be used for this agent session.
    pub model_override: Option<ModelOverride>,
    /// Override the capabilities for this agent session.
    pub capability_override: Option<CapabilityOverride>,
    /// New messages for the agent's conversation.
    pub messages: Vec<Message>,
    /// Initial context variables for the agent.
    pub context_vars: Vec<ContextVar>,
    /// The tools available to the agent.
    pub tools: Vec<ToolDefinition>,
    /// Additional arbitrary context that can be used by the agent or tools, not structured as variables.
    pub context: Value,
}

impl Default for ReActStateInput {
    fn default() -> Self {
        Self {
            model_override: None,
            capability_override: None,
            run_id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            messages: Vec::new(),
            context_vars: Vec::new(),
            tools: Vec::new(),
            context: Value::Object(Default::default()),
        }
    }
}

impl IntoStateUpdate<ReActStateUpdate> for ReActStateInput {
    fn into_update(self) -> Result<Option<ReActStateUpdate>, GraphError> {
        Ok(Some(ReActStateUpdate {
            messages: self.messages,
            // Convert the context Value into RFC 6902 `add` operations, one per
            // top-level key. `add` replaces an existing key or inserts a missing
            // one, giving shallow-merge semantics without needing the current state.
            // Non-object values produce no operations.
            context: match self.context {
                Value::Object(map) => map
                    .into_iter()
                    .filter_map(|(key, value)| {
                        // RFC 6901: `~` -> `~0`, `/` -> `~1`
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
        }))
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
    }

    fn merge(mut self, other: Self) -> Self {
        self.messages.extend(other.messages);
        self.context.extend(other.context);
        self
    }
}

impl GraphStateInput for ReActStateInput {
    type State = ReActState;

    fn initialize(self) -> Self::State {
        ReActState {
            model_override: self.model_override,
            capability_override: self.capability_override,
            run_id: self.run_id,
            session_id: self.session_id,
            messages: self.messages,
            context_vars: self.context_vars,
            tools: self.tools,
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
        }
        .merge(ReActStateUpdate {
            messages: vec![],
            context: vec![PatchOperation::Add(AddOperation {
                path: "/b".try_into().unwrap(),
                value: json!(2),
            })],
        })
        .apply(&mut state);

        assert_eq!(state.context["a"], json!(1));
        assert_eq!(state.context["b"], json!(2));
    }
}
