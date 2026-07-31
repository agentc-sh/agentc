// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use serde::Serialize;

use crate::semconv::genai::GenAiPart;

/// Roles supported by OpenTelemetry GenAI messages.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GenAiRole {
    System,
    User,
    Assistant,
    Tool,
}

/// An OpenTelemetry GenAI input message.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GenAiMessage {
    role: GenAiRole,
    parts: Vec<GenAiPart>,
}

impl GenAiMessage {
    /// Creates a system message.
    pub fn system(parts: impl IntoIterator<Item = GenAiPart>) -> Self {
        Self::new(GenAiRole::System, parts)
    }

    /// Creates a user message.
    pub fn user(parts: impl IntoIterator<Item = GenAiPart>) -> Self {
        Self::new(GenAiRole::User, parts)
    }

    /// Creates an assistant message.
    pub fn assistant(parts: impl IntoIterator<Item = GenAiPart>) -> Self {
        Self::new(GenAiRole::Assistant, parts)
    }

    /// Creates a tool message.
    pub fn tool(parts: impl IntoIterator<Item = GenAiPart>) -> Self {
        Self::new(GenAiRole::Tool, parts)
    }

    fn new(role: GenAiRole, parts: impl IntoIterator<Item = GenAiPart>) -> Self {
        Self { role, parts: parts.into_iter().collect() }
    }
}

/// An OpenTelemetry GenAI output message candidate.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GenAiOutputMessage {
    role: GenAiRole,
    parts: Vec<GenAiPart>,
    finish_reason: String,
}

impl GenAiOutputMessage {
    /// Creates an output candidate from a message and its finish reason.
    pub fn new(message: GenAiMessage, finish_reason: impl Into<String>) -> Self {
        Self {
            role: message.role,
            parts: message.parts,
            finish_reason: finish_reason.into(),
        }
    }
}

/// OpenTelemetry GenAI system instructions.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GenAiSystemInstructions(Vec<GenAiPart>);

impl GenAiSystemInstructions {
    /// Creates system instructions from ordered message parts.
    pub fn new(parts: impl IntoIterator<Item = GenAiPart>) -> Self {
        Self(parts.into_iter().collect())
    }

    /// Serializes the system instructions to their attribute value.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

/// OpenTelemetry GenAI input messages.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GenAiInputMessages(Vec<GenAiMessage>);

impl GenAiInputMessages {
    /// Creates input messages from an ordered message sequence.
    pub fn new(messages: impl IntoIterator<Item = GenAiMessage>) -> Self {
        Self(messages.into_iter().collect())
    }

    /// Extends the ordered input message sequence.
    pub fn extend(&mut self, messages: impl IntoIterator<Item = GenAiMessage>) {
        self.0.extend(messages);
    }

    /// Returns whether the input message sequence is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Serializes the input messages to their attribute value.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

/// OpenTelemetry GenAI output message candidates.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GenAiOutputMessages(Vec<GenAiOutputMessage>);

impl GenAiOutputMessages {
    /// Creates output messages from an ordered candidate sequence.
    pub fn new(messages: impl IntoIterator<Item = GenAiOutputMessage>) -> Self {
        Self(messages.into_iter().collect())
    }

    /// Returns whether the output message sequence is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Serializes the output messages to their attribute value.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::semconv::genai::{GenAiInputMessages, GenAiMessage, GenAiPart};

    #[test]
    fn input_messages_serialize_structured_parts() {
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(
                &GenAiInputMessages::new([
                    GenAiMessage::user([GenAiPart::text("Use the calculator")]),
                    GenAiMessage::assistant([GenAiPart::tool_call(
                        "call-1",
                        "calculator",
                        json!({ "expression": "42 + 7" }),
                    )]),
                    GenAiMessage::tool([GenAiPart::tool_call_response(
                        "call-1",
                        json!({ "result": 49 }),
                    )]),
                ])
                .to_json()
                .unwrap(),
            )
            .unwrap(),
            json!([
                {
                    "role": "user",
                    "parts": [
                        {
                            "type": "text",
                            "content": "Use the calculator",
                        },
                    ],
                },
                {
                    "role": "assistant",
                    "parts": [
                        {
                            "type": "tool_call",
                            "id": "call-1",
                            "name": "calculator",
                            "arguments": {
                                "expression": "42 + 7",
                            },
                        },
                    ],
                },
                {
                    "role": "tool",
                    "parts": [
                        {
                            "type": "tool_call_response",
                            "id": "call-1",
                            "response": {
                                "result": 49,
                            },
                        },
                    ],
                },
            ]),
        );
    }
}
