// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

#![allow(deprecated)]

use serde::Serialize;
use serde_json::Value;

use agentc_telemetry::{
    Span,
    semconv::{
        self, attribute,
        genai::{GenAiInputMessages, GenAiOutputMessages},
    },
};

use crate::graph::{
    runtime::RunOutcome,
    state::{GraphNode, InputOf, StateOf},
};

/// Provides the portable OpenTelemetry GenAI messages for an agent graph.
pub trait InvokeAgentSpans: GraphNode {
    /// Projects an invocation input into portable messages. `None` means that
    /// no portable input messages are available.
    fn input_messages(
        input: &InputOf<Self>,
    ) -> Result<Option<GenAiInputMessages>, serde_json::Error>;

    /// Projects a completed invocation state into portable messages. `None`
    /// means that no portable output messages are available.
    fn output_messages(
        state: &StateOf<Self>,
    ) -> Result<Option<GenAiOutputMessages>, serde_json::Error>;
}

pub(crate) struct InvokeAgentSpan;

impl InvokeAgentSpan {
    pub(crate) fn record_input<N>(
        span: &Span,
        input: &InputOf<N>,
    ) where
        N: InvokeAgentSpans,
    {
        if let Ok(input) = serde_json::to_string(input) {
            span.record(semconv::AGENTC_AGENT_INPUT, input.as_str());
        }

        if let Ok(Some(messages)) = N::input_messages(input)
            && let Ok(messages) = messages.to_json()
        {
            span.record(attribute::GEN_AI_INPUT_MESSAGES, messages.as_str());
        }
    }

    pub(crate) fn record_output<N>(
        span: &Span,
        outcome: &RunOutcome<StateOf<N>>,
    ) where
        N: InvokeAgentSpans,
    {
        if let Ok(output) = serde_json::to_string(&AgentOutputAttribute::new(outcome)) {
            span.record(semconv::AGENTC_AGENT_OUTPUT, output.as_str());
        }

        if let RunOutcome::Completed(state) = outcome
            && let Ok(Some(messages)) = N::output_messages(state)
            && let Ok(messages) = messages.to_json()
        {
            span.record(attribute::GEN_AI_OUTPUT_MESSAGES, messages.as_str());
        }
    }
}

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum AgentOutputAttribute<'a, S> {
    Completed {
        state: &'a S,
    },
    Interrupted {
        state: &'a S,
        #[serde(skip_serializing_if = "Option::is_none")]
        interrupt_payload: Option<&'a Value>,
    },
    Cancelled {
        state: &'a S,
    },
}

impl<'a, S> AgentOutputAttribute<'a, S> {
    fn new(outcome: &'a RunOutcome<S>) -> Self {
        match outcome {
            RunOutcome::Completed(state) => Self::Completed { state },
            RunOutcome::Interrupted { state, payload } => Self::Interrupted {
                state,
                interrupt_payload: payload.as_ref(),
            },
            RunOutcome::Cancelled { state } => Self::Cancelled { state },
        }
    }
}

#[cfg(test)]
mod tests {
    use serde::Serialize;
    use serde_json::json;

    use crate::{
        graph::runtime::RunOutcome,
        instrument::AgentOutputAttribute,
    };

    #[derive(Serialize)]
    struct TestState {
        value: u32,
    }

    #[test]
    fn completed_output_preserves_terminal_state() {
        assert_eq!(
            serde_json::to_value(AgentOutputAttribute::new(&RunOutcome::completed(
                TestState { value: 42 },
            )))
            .unwrap(),
            json!({
                "status": "completed",
                "state": {
                    "value": 42,
                },
            }),
        );
    }

    #[test]
    fn interrupted_output_preserves_state_and_payload() {
        assert_eq!(
            serde_json::to_value(AgentOutputAttribute::new(&RunOutcome::interrupted(
                TestState { value: 42 },
                Some(json!({ "reason": "approval" })),
            )))
            .unwrap(),
            json!({
                "status": "interrupted",
                "state": {
                    "value": 42,
                },
                "interrupt_payload": {
                    "reason": "approval",
                },
            }),
        );
    }

    #[test]
    fn interrupted_output_omits_absent_payload() {
        assert_eq!(
            serde_json::to_value(AgentOutputAttribute::new(&RunOutcome::interrupted(
                TestState { value: 42 },
                None,
            )))
            .unwrap(),
            json!({
                "status": "interrupted",
                "state": {
                    "value": 42,
                },
            }),
        );
    }

    #[test]
    fn cancelled_output_preserves_terminal_state() {
        assert_eq!(
            serde_json::to_value(AgentOutputAttribute::new(&RunOutcome::cancelled(
                TestState { value: 42 },
            )))
            .unwrap(),
            json!({
                "status": "cancelled",
                "state": {
                    "value": 42,
                },
            }),
        );
    }
}
