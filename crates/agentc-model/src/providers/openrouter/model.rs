// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use futures::StreamExt;
use rig_core::{
    client::CompletionClient, completion::CompletionModel as RigCompletionModel, message::Message,
    providers::openrouter,
};
use serde_json::json;

use crate::{
    errors::ModelError,
    providers::openrouter::constants::{OTEL_PROVIDER_NAME, PROVIDER},
    stream::ChatCompletionStream,
    traits::CompletionModel,
    types::{
        identity::{ModelId, ProviderId},
        inference::InferenceParams,
        request::CompletionRequest,
    },
};

/// A specific OpenRouter model instance. Obtained from
/// [`OpenRouterClient::model`](crate::providers::openrouter::client::OpenRouterClient::model).
pub struct OpenRouterModel {
    model: openrouter::CompletionModel,
    model_id: ModelId,
    inference_params: InferenceParams,
}

impl OpenRouterModel {
    pub fn new(
        client: openrouter::Client,
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
impl CompletionModel for OpenRouterModel {
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

        if let Some(stop_sequences) = request.stop_sequences {
            additional["stop"] = json!(stop_sequences);
        }

        if let Some(frequency_penalty) = request.frequency_penalty {
            additional["frequency_penalty"] = json!(frequency_penalty);
        }

        if let Some(presence_penalty) = request.presence_penalty {
            additional["presence_penalty"] = json!(presence_penalty);
        }

        if let Some(seed) = request.seed {
            additional["seed"] = json!(seed);
        }

        // top_k is not supported by OpenRouter and is intentionally ignored here.

        if additional
            .as_object()
            .is_some_and(|m| !m.is_empty())
        {
            builder = builder.additional_params(additional);
        }

        Ok(ChatCompletionStream::new(
            builder
                .messages(
                    rest.into_iter()
                        .map(TryInto::try_into)
                        .collect::<Result<Vec<_>, _>>()?,
                )
                .stream()
                .await
                .map_err(|e| ModelError::provider(PROVIDER, e.to_string(), Some(e)))?
                .filter_map(|event| async move {
                    match event {
                        Ok(e) => Some(Ok(e.try_into().ok()?)),
                        Err(e) => Some(Err(ModelError::stream(e.to_string(), Some(e)))),
                    }
                }),
        ))
    }
}
