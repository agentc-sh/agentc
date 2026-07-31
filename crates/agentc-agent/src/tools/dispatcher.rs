// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

#![allow(deprecated)]

use futures::future::join_all;
use serde::Serialize;
use serde_json::Value;
use std::{
    sync::{Arc, LazyLock},
    time::Instant,
};

use agentc_telemetry::{
    Instrument, field, info_span,
    metrics::{Histogram, KeyValue, meter},
    semconv::{self, attribute},
};

use crate::{
    graph::state::{AnyState, GraphState},
    tools::{
        activity::ActivityEmitter,
        registry::ToolRegistry,
        types::{ToolExecutionContext, ToolInput, ToolResponse},
    },
    types::tools::ToolCall,
};

static EXECUTE_TOOL_DURATION: LazyLock<Histogram<f64>> = LazyLock::new(|| {
    meter("agentc-agent")
        .f64_histogram(semconv::GEN_AI_EXECUTE_TOOL_DURATION)
        .with_unit("s")
        .with_description("Duration of tool executions.")
        .build()
});

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

#[derive(Serialize)]
struct ToolResultAttribute<'a> {
    value: &'a Value,
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
        context: ToolExecutionContext,
        emitter: Option<ActivityEmitter>,
    ) -> DispatchOutcome<S::Update>
    where
        S: GraphState + 'static,
    {
        let any_state: Arc<dyn AnyState> = Arc::new(state.clone());

        let start = Instant::now();
        let attributes = vec![
            KeyValue::new(attribute::GEN_AI_TOOL_NAME, call.name.clone()),
            KeyValue::new(attribute::GEN_AI_TOOL_TYPE, "extension"),
        ];

        if let Some(tool) = self.registry.get(&call.name) {
            let span = info_span!(
                "execute_tool",
                otel.name = %format!("execute_tool {}", call.name),
                otel.kind = "internal",
                gen_ai.operation.name = "execute_tool",
                gen_ai.tool.name = %call.name,
                gen_ai.tool.call.id = %call.id,
                gen_ai.tool.type = "extension",
                gen_ai.tool.call.arguments = field::Empty,
                gen_ai.tool.call.result = field::Empty,
                error.type = field::Empty,
            );

            if let Ok(arguments) = serde_json::to_string(&call.arguments) {
                span.record(attribute::GEN_AI_TOOL_CALL_ARGUMENTS, arguments.as_str());
            }

            match tool
                .execute(
                    ToolInput::new(call.arguments, context)
                        .with_state(any_state)
                        .maybe_with_activity_emitter(emitter),
                )
                .instrument(span.clone())
                .await
            {
                ToolResponse::Success { content, state_update } => {
                    if let Ok(value) =
                        serde_json::to_string(&ToolResultAttribute { value: &content })
                    {
                        span.record(attribute::GEN_AI_TOOL_CALL_RESULT, value.as_str());
                    }

                    EXECUTE_TOOL_DURATION.record(start.elapsed().as_secs_f64(), &attributes);

                    return DispatchOutcome::Success {
                        call_id: call.id,
                        content,
                        state_update: state_update
                            .and_then(|boxed| boxed.downcast::<S::Update>().ok()),
                    };
                }
                ToolResponse::Error { message } => {
                    span.record("error.type", "tool_error");

                    let mut attributes = attributes;
                    attributes.push(KeyValue::new(attribute::ERROR_TYPE, "tool_error"));
                    EXECUTE_TOOL_DURATION.record(start.elapsed().as_secs_f64(), &attributes);

                    return DispatchOutcome::Error { call_id: call.id, error: message };
                }
            }
        }

        if self.client_tools.contains(&call.name) {
            return DispatchOutcome::AwaitingClient(call);
        }

        let mut attributes = attributes;
        attributes.push(KeyValue::new(attribute::ERROR_TYPE, "not_found"));
        EXECUTE_TOOL_DURATION.record(start.elapsed().as_secs_f64(), &attributes);

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
        context: ToolExecutionContext,
    ) -> Vec<DispatchOutcome<S::Update>>
    where
        S: GraphState + 'static,
    {
        join_all(
            calls
                .into_iter()
                .map(|(call, emitter)| self.dispatch(call, state, context.clone(), emitter)),
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

#[cfg(test)]
mod tests {
    use super::*;

    use serde_json::{from_str, json};

    #[test]
    fn tool_result_attribute_preserves_every_json_shape() {
        for result in [
            json!({"value": 42}),
            json!(["one", "two"]),
            json!("text"),
            json!(42),
            json!(true),
            Value::Null,
        ] {
            assert_eq!(
                from_str::<Value>(
                    &serde_json::to_string(&ToolResultAttribute { value: &result },)
                        .expect("tool result should serialize"),
                )
                .expect("tool result JSON should parse"),
                json!({
                    "value": result
                }),
            );
        }
    }
}
