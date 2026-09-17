// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use agentc_agent::{instrument::InvokeAgentSpans, types::conversion::ToModelType};
use agentc_model::types::message::ChatMessage;
use agentc_telemetry::semconv::genai::{
    GenAiInputMessages, GenAiOutputMessage, GenAiOutputMessages, ToGenAiType,
};

use crate::{
    graph::{
        runtime::ReActNode,
        state::{ReActState, ReActStateInput},
    },
    types::message::{Message, MessageList},
};

impl InvokeAgentSpans for ReActNode {
    fn input_messages(
        input: &ReActStateInput,
    ) -> Result<Option<GenAiInputMessages>, serde_json::Error> {
        let messages = GenAiInputMessages::new(
            MessageList::new(input.messages.clone())
                .to_model_type()
                .into_iter()
                .map(|message| message.to_gen_ai_type())
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .flatten(),
        );

        Ok((!messages.is_empty()).then_some(messages))
    }

    fn output_messages(
        state: &ReActState,
    ) -> Result<Option<GenAiOutputMessages>, serde_json::Error> {
        let Some((index, assistant)) = state
            .messages
            .iter()
            .enumerate()
            .rev()
            .find_map(|(index, message)| match message {
                Message::Assistant(assistant) if assistant.run_id == state.run_id => {
                    Some((index, assistant))
                }
                _ => None,
            })
        else {
            return Ok(None);
        };

        let mut messages = Vec::new();

        if let Some(Message::Reasoning(reasoning)) = index
            .checked_sub(1)
            .and_then(|index| state.messages.get(index))
            && reasoning.run_id == state.run_id
        {
            messages.push(Message::Reasoning(reasoning.clone()));
        }

        messages.push(Message::Assistant(assistant.clone()));

        let messages = MessageList::new(messages).to_model_type();
        let [ChatMessage::Assistant(message)] = messages.as_slice() else {
            return Ok(None);
        };

        Ok(Some(GenAiOutputMessages::new([GenAiOutputMessage::new(
            message.to_gen_ai_type()?,
            if assistant.has_tool_calls() {
                "tool_call"
            } else {
                "stop"
            },
        )])))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, from_str, json};
    use uuid::Uuid;

    use agentc_agent::{
        graph::state::GraphStateInput, instrument::InvokeAgentSpans, types::tools::ToolCall,
    };

    use crate::{
        graph::{runtime::ReActNode, state::ReActStateInput},
        types::message::{AssistantMessage, Message, ReasoningMessage},
    };

    #[test]
    fn input_messages_preserve_conversation_order() {
        assert_eq!(
            from_str::<Value>(
                &ReActNode::input_messages(&ReActStateInput {
                    messages: vec![
                        Message::user("Calculate a value"),
                        Message::Assistant(AssistantMessage::new().with_tool_calls(vec![
                            ToolCall {
                                id: "call-1".to_string(),
                                name: "calculate".to_string(),
                                arguments: json!({ "value": 21 }),
                            },
                        ])),
                        Message::tool("call-1", "calculate", "42"),
                    ],
                    ..Default::default()
                })
                .expect("input messages should convert")
                .expect("input messages should exist")
                .to_json()
                .expect("input messages should serialize"),
            )
            .expect("input JSON should parse"),
            json!([
                {
                    "role": "user",
                    "parts": [
                        {
                            "type": "text",
                            "content": "Calculate a value",
                        },
                    ],
                },
                {
                    "role": "assistant",
                    "parts": [
                        {
                            "type": "tool_call",
                            "id": "call-1",
                            "name": "calculate",
                            "arguments": {
                                "value": 21,
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
                            "response": "42",
                        },
                    ],
                },
            ]),
        );
    }

    #[test]
    fn output_messages_select_current_response_and_visible_reasoning() {
        let run_id = Uuid::new_v4();

        assert_eq!(
            from_str::<Value>(
                &ReActNode::output_messages(
                    &ReActStateInput {
                        run_id,
                        messages: vec![
                            Message::assistant("historical").with_run_id(Uuid::from_u128(1)),
                            Message::Reasoning(
                                ReasoningMessage::new("visible reasoning")
                                    .with_run_id(run_id)
                                    .with_signature("opaque-signature"),
                            ),
                            Message::assistant("final answer").with_run_id(run_id),
                        ],
                        ..Default::default()
                    }
                    .initialize(),
                )
                .expect("output messages should convert")
                .expect("output messages should exist")
                .to_json()
                .expect("output messages should serialize"),
            )
            .expect("output JSON should parse"),
            json!([
                {
                    "role": "assistant",
                    "parts": [
                        {
                            "type": "reasoning",
                            "content": "visible reasoning",
                        },
                        {
                            "type": "text",
                            "content": "final answer",
                        },
                    ],
                    "finish_reason": "stop",
                },
            ]),
        );
    }

    #[test]
    fn output_messages_mark_tool_calls() {
        let run_id = Uuid::new_v4();

        assert_eq!(
            from_str::<Value>(
                &ReActNode::output_messages(
                    &ReActStateInput {
                        run_id,
                        messages: vec![Message::Assistant(
                            AssistantMessage::new()
                                .with_run_id(run_id)
                                .with_tool_calls(vec![ToolCall {
                                    id: "call-1".to_string(),
                                    name: "calculate".to_string(),
                                    arguments: json!({ "value": 42 }),
                                }]),
                        )],
                        ..Default::default()
                    }
                    .initialize(),
                )
                .expect("output messages should convert")
                .expect("output messages should exist")
                .to_json()
                .expect("output messages should serialize"),
            )
            .expect("output JSON should parse"),
            json!([
                {
                    "role": "assistant",
                    "parts": [
                        {
                            "type": "tool_call",
                            "id": "call-1",
                            "name": "calculate",
                            "arguments": {
                                "value": 42,
                            },
                        },
                    ],
                    "finish_reason": "tool_call",
                },
            ]),
        );
    }

    #[test]
    fn output_messages_omit_other_runs() {
        assert!(
            ReActNode::output_messages(
                &ReActStateInput {
                    run_id: Uuid::from_u128(1),
                    messages: vec![
                        Message::assistant("historical").with_run_id(Uuid::from_u128(2))
                    ],
                    ..Default::default()
                }
                .initialize(),
            )
            .expect("output messages should convert")
            .is_none(),
        );
    }
}
