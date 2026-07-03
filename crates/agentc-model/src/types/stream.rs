// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use crate::types::{reasoning::Reasoning, tools::ToolCall, usage::TokenUsage};

/// The tool call delta content.
#[derive(Debug, Clone)]
pub enum ToolCallDelta {
    /// The name of the tool being called. Only present on the first delta for a given tool call id.
    Name(String),
    /// A partial or full update to the tool call arguments.
    Arguments(String),
}

/// A single event yielded by a [`ChatCompletionStream`](crate::stream::ChatCompletionStream).
#[derive(Debug, Clone)]
pub enum CompletionStreamEvent {
    /// A text content delta from the model.
    TextDelta { delta: String },

    /// A reasoning or thinking token delta, for models that expose it.
    ReasoningDelta { id: Option<String>, delta: String },

    /// A fully assembled reasoning or thinking chunk, emitted once
    /// all its deltas have arrived.
    Reasoning(Reasoning),

    /// Streaming arguments for an in-progress tool call.
    ToolCallDelta { id: String, delta: ToolCallDelta },

    /// A fully assembled tool call, emitted once all its argument
    /// deltas have arrived.
    ToolCall(ToolCall),

    /// Final token usage stats. Always the last event in the stream.
    Done(TokenUsage),
}
