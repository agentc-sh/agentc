// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use futures::StreamExt;
use rig_core::{
    client::CompletionClient, completion::CompletionModel as RigCompletionModel, message::Message,
    providers::gemini,
};
use serde_json::json;

use crate::{
    errors::{IntoModelError, ModelError},
    providers::{
        gemini::constants::{OTEL_PROVIDER_NAME, PROVIDER},
        rig::events::CompletionStreamMetadata,
    },
    stream::ChatCompletionStream,
    traits::CompletionModel,
    types::{
        identity::{ModelId, ProviderId},
        inference::InferenceParams,
        request::CompletionRequest,
    },
};

impl CompletionStreamMetadata for gemini::streaming::StreamingCompletionResponse {
    fn finish_reason(&self) -> Option<String> {
        self.finish_reason.as_ref().and_then(|reason| {
            serde_json::to_value(reason)
                .ok()?
                .as_str()
                .map(|reason| match reason {
                    "STOP" => "stop".to_string(),
                    "MAX_TOKENS" => "length".to_string(),
                    "SAFETY"
                    | "RECITATION"
                    | "LANGUAGE"
                    | "BLOCKLIST"
                    | "PROHIBITED_CONTENT"
                    | "SPII" => "content_filter".to_string(),
                    reason => reason.to_lowercase(),
                })
        })
    }
}

/// A specific Gemini model instance. Obtained from
/// [`GeminiClient::model`](crate::providers::gemini::client::GeminiClient::model).
pub struct GeminiModel {
    model: gemini::completion::CompletionModel,
    model_id: ModelId,
    inference_params: InferenceParams,
}

impl GeminiModel {
    pub fn new(
        client: gemini::Client,
        model_id: ModelId,
        inference_params: InferenceParams,
    ) -> Self {
        Self {
            model: client.completion_model(model_id.as_str()),
            model_id,
            inference_params,
        }
    }
}

#[async_trait]
impl CompletionModel for GeminiModel {
    fn provider(&self) -> ProviderId {
        PROVIDER.into()
    }

    fn otel_provider_name(&self) -> &'static str {
        OTEL_PROVIDER_NAME
    }

    fn model(&self) -> &ModelId {
        &self.model_id
    }

    fn inference_params(&self) -> &InferenceParams {
        &self.inference_params
    }

    async fn send(&self, request: CompletionRequest) -> Result<ChatCompletionStream, ModelError> {
        let request = request.with_defaults(&self.inference_params);
        let (system, latest, rest) = request.messages.split()?;
        let mut builder = self
            .model
            .completion_request(Message::try_from(latest)?);
        let mut additional = json!(
            request
                .provider_params
                .unwrap_or_default()
        );

        if let Some(system_prompt) = system {
            builder = builder.preamble(system_prompt);
        }

        if let Ok(tools) = request
            .tools
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<_>, _>>()
            && !tools.is_empty()
        {
            builder = builder.tools(tools);
        }

        if let Some(max_tokens) = request.max_tokens {
            builder = builder.max_tokens(max_tokens);
        }

        if let Some(temperature) = request.temperature {
            builder = builder.temperature(temperature);
        }

        if let Some(top_p) = request.top_p {
            additional["top_p"] = json!(top_p);
        }

        if let Some(top_k) = request.top_k {
            additional["top_k"] = json!(top_k);
        }

        if let Some(stop_sequences) = request.stop_sequences {
            additional["stop_sequences"] = json!(stop_sequences);
        }

        if let Some(seed) = request.seed {
            additional["seed"] = json!(seed);
        }

        // frequency_penalty and presence_penalty are not supported by Gemini.

        if additional
            .as_object()
            .is_some_and(|m| !m.is_empty())
        {
            builder = builder.additional_params(additional);
        }

        ChatCompletionStream::establish(
            builder
                .messages(
                    rest.into_iter()
                        .map(TryInto::try_into)
                        .collect::<Result<Vec<_>, _>>()?,
                )
                .stream()
                .await
                .map_err(|e| e.into_model_error(PROVIDER))?
                .filter_map(|event| async move {
                    match event {
                        Ok(e) => Some(Ok(e.try_into().ok()?)),
                        Err(e) => Some(Err(e.into_model_error(PROVIDER))),
                    }
                }),
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use rig_core::{
        providers::gemini::{
            completion::gemini_api_types::FinishReason,
            streaming::{PartialUsage, StreamingCompletionResponse},
        },
        streaming::StreamedAssistantContent,
    };

    use crate::types::stream::CompletionStreamEvent;

    #[test]
    fn final_response_preserves_usage_and_finish_reason() {
        let event = CompletionStreamEvent::try_from(StreamedAssistantContent::Final(
            StreamingCompletionResponse {
                usage_metadata: PartialUsage {
                    total_token_count: 15,
                    cached_content_token_count: Some(2),
                    candidates_token_count: Some(5),
                    prompt_token_count: 10,
                    ..Default::default()
                },
                finish_reason: Some(FinishReason::Stop),
                finish_message: None,
                model_version: None,
            },
        ))
        .expect("final response should convert");

        let CompletionStreamEvent::Done(final_response) = event else {
            panic!("expected final completion metadata");
        };

        assert_eq!(final_response.usage.input_tokens, 10);
        assert_eq!(final_response.usage.output_tokens, 5);
        assert_eq!(final_response.usage.cache_input_tokens, Some(2));
        assert_eq!(final_response.finish_reason.as_deref(), Some("stop"));
    }
}
