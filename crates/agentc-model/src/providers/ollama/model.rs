// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use futures::StreamExt;
use rig_core::{
    client::CompletionClient, completion::CompletionModel as RigCompletionModel, message::Message,
    providers::ollama,
};
use serde_json::json;

use crate::{
    errors::{IntoModelError, ModelError},
    providers::ollama::constants::{OTEL_PROVIDER_NAME, PROVIDER},
    stream::ChatCompletionStream,
    traits::CompletionModel,
    types::{
        identity::{ModelId, ProviderId},
        inference::InferenceParams,
        request::CompletionRequest,
    },
};

/// A specific Ollama model instance. Obtained from
/// [`OllamaClient::model`](crate::providers::ollama::OllamaClient::model).
pub struct OllamaModel {
    model: ollama::CompletionModel,
    model_id: ModelId,
    inference_params: InferenceParams,
}

impl OllamaModel {
    pub fn new(
        client: ollama::Client,
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
impl CompletionModel for OllamaModel {
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

        if let Some(stop_sequences) = request.stop_sequences {
            additional["stop"] = json!(stop_sequences);
        }

        // seed: prefer the cross-provider field; fall back to Ollama-specific params.
        if let Some(seed) = request.seed {
            additional["seed"] = json!(seed);
        }

        // top_p, top_k, frequency_penalty, and presence_penalty are not forwarded:
        // Ollama does not support them in its native completion API format.

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
