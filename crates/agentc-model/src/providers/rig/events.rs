// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use rig::completion::GetTokenUsage;
use rig::streaming::{StreamedAssistantContent, ToolCallDeltaContent};

use crate::{
    errors::ModelError,
    types::{
        reasoning::{Reasoning, ReasoningContent},
        stream::{CompletionStreamEvent, ToolCallDelta},
        tools::ToolCall,
        usage::TokenUsage,
    },
};

impl TryFrom<ToolCallDeltaContent> for ToolCallDelta {
    type Error = ModelError;

    fn try_from(value: ToolCallDeltaContent) -> Result<Self, Self::Error> {
        match value {
            ToolCallDeltaContent::Name(name) => Ok(ToolCallDelta::Name(name)),
            ToolCallDeltaContent::Delta(delta) => Ok(ToolCallDelta::Arguments(delta)),
        }
    }
}

impl<G> TryFrom<StreamedAssistantContent<G>> for CompletionStreamEvent
where
    G: GetTokenUsage,
{
    type Error = ModelError;

    fn try_from(value: StreamedAssistantContent<G>) -> Result<Self, Self::Error> {
        match value {
            StreamedAssistantContent::Text(text) => {
                Ok(CompletionStreamEvent::TextDelta { delta: text.text })
            }
            StreamedAssistantContent::Reasoning(reasoning) => {
                Ok(CompletionStreamEvent::Reasoning(Reasoning {
                    id: reasoning.id,
                    content: reasoning
                        .content
                        .into_iter()
                        .map(ReasoningContent::try_from)
                        .collect::<Result<Vec<_>, _>>()?,
                }))
            }
            StreamedAssistantContent::ReasoningDelta { id, reasoning } => {
                Ok(CompletionStreamEvent::ReasoningDelta { id, delta: reasoning })
            }
            StreamedAssistantContent::ToolCall { tool_call, .. } => {
                Ok(CompletionStreamEvent::ToolCall(ToolCall {
                    id: tool_call.id,
                    name: tool_call.function.name,
                    arguments: tool_call.function.arguments,
                }))
            }
            StreamedAssistantContent::ToolCallDelta { id, content, .. } => {
                Ok(CompletionStreamEvent::ToolCallDelta { id, delta: content.try_into()? })
            }
            StreamedAssistantContent::Final(response) => {
                let usage = response.token_usage();

                Ok(CompletionStreamEvent::Done(TokenUsage {
                    input_tokens: usage.map_or(0, |u| u.input_tokens as u32),
                    output_tokens: usage.map_or(0, |u| u.output_tokens as u32),
                    cache_input_tokens: usage.map(|u| u.cached_input_tokens as u32),
                }))
            }
        }
    }
}
