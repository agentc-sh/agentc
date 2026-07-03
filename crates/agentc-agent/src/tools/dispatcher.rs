// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use futures::future::join_all;
use serde_json::Value;
use std::sync::Arc;

use crate::{
    graph::state::{AnyState, GraphState},
    tools::{
        activity::ActivityEmitter,
        registry::ToolRegistry,
        types::{ToolInput, ToolResponse},
    },
    types::tools::ToolCall,
};

#[derive(Debug)]
pub enum DispatchOutcome<U> {
    /// The tool was invoked, result is ready
    Success {
        call_id: String,
        content: Value,
        state_update: Option<U>,
    },
    /// The client should handle this tool call, wait for a response
    AwaitingClient(ToolCall),
    /// Tool not found anywhere
    NotFound { call_id: String, name: String },
    /// Tool execution failed with an error
    Error { call_id: String, error: String },
}

/// A struct responsible for dispatching tool calls to the appropriate tool implementations.
pub struct ToolDispatcher {
    registry: ToolRegistry,
    client_tools: Vec<String>,
}

impl ToolDispatcher {
    /// Create a new [`ToolDispatcher`](crate::tools::dispatcher::ToolDispatcher) with a reference to the tool registry.
    pub fn new(registry: ToolRegistry) -> Self {
        Self { registry, client_tools: Vec::new() }
    }

    /// Add client tools to pause on.
    pub fn with_client_tools(mut self, tools: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.client_tools = tools
            .into_iter()
            .map(Into::into)
            .collect();
        self
    }

    /// Invoke the tool for a given [`ToolCall`](crate::types::tools::ToolCall).
    ///
    /// `emitter` is an optional [`ActivityEmitter`](crate::tools::activity::ActivityEmitter)
    /// created by the caller. Pass `Some(emitter)` to receive incremental activity deltas
    /// from the tool during execution, or `None` to opt out.
    pub async fn dispatch<S>(
        &self,
        call: ToolCall,
        state: &S,
        emitter: Option<ActivityEmitter>,
    ) -> DispatchOutcome<S::Update>
    where
        S: GraphState + 'static,
    {
        let any_state: Arc<dyn AnyState> = Arc::new(state.clone());

        if let Some(tool) = self.registry.get(&call.name) {
            match tool
                .execute(
                    ToolInput::new(call.arguments)
                        .with_state(any_state)
                        .maybe_with_activity_emitter(emitter),
                )
                .await
            {
                ToolResponse::Success { content, state_update } => {
                    return DispatchOutcome::Success {
                        call_id: call.id,
                        content,
                        state_update: state_update
                            .and_then(|boxed| boxed.downcast::<S::Update>().ok()),
                    };
                }
                ToolResponse::Error { message } => {
                    return DispatchOutcome::Error { call_id: call.id, error: message };
                }
            }
        }

        if self.client_tools.contains(&call.name) {
            return DispatchOutcome::AwaitingClient(call);
        }

        DispatchOutcome::NotFound { call_id: call.id, name: call.name }
    }

    /// Invoke the tools for multiple [`ToolCall`](crate::types::tools::ToolCall)s.
    ///
    /// Each entry pairs a [`ToolCall`](crate::types::tools::ToolCall) with an optional
    /// [`ActivityEmitter`](crate::tools::activity::ActivityEmitter). Pass `Some(emitter)`
    /// for calls that should stream activity deltas, or `None` to opt out per call.
    pub async fn dispatch_all<S>(
        &self,
        calls: Vec<(ToolCall, Option<ActivityEmitter>)>,
        state: &S,
    ) -> Vec<DispatchOutcome<S::Update>>
    where
        S: GraphState + 'static,
    {
        join_all(
            calls
                .into_iter()
                .map(|(call, emitter)| self.dispatch(call, state, emitter)),
        )
        .await
    }
}

pub trait ToolRegistryExt {
    fn dispatcher(&self) -> ToolDispatcher;
}

impl ToolRegistryExt for ToolRegistry {
    fn dispatcher(&self) -> ToolDispatcher {
        ToolDispatcher::new(self.clone())
    }
}
