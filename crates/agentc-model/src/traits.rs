// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, from_value, to_value};
use std::sync::Arc;

use crate::{
    errors::ModelError,
    middleware::{CompletionMiddleware, Intercepted},
    stream::ChatCompletionStream,
    types::{
        identity::{ModelId, ProviderId},
        inference::InferenceParams,
        message::ChatMessage,
        request::CompletionRequest,
        tools::ToolSpec,
    },
};

/// A model instance representing a specific model from a provider, capable of streaming completions.
#[async_trait]
pub trait CompletionModel: Send + Sync {
    /// The provider this model belongs to.
    fn provider(&self) -> ProviderId;
    /// The OTel `gen_ai.provider.name` value for this provider.
    fn otel_provider_name(&self) -> &'static str;
    /// The model name this instance was created with.
    fn model(&self) -> &ModelId;
    /// The inference parameter defaults baked into this model instance at construction time.
    /// Request-level params take precedence over these when building a completion request.
    fn inference_params(&self) -> &InferenceParams;

    /// Send a fully constructed request to the provider and return its raw streaming response.
    async fn send(&self, request: CompletionRequest) -> Result<ChatCompletionStream, ModelError>;
}

#[async_trait]
impl CompletionModel for Arc<dyn CompletionModel> {
    fn provider(&self) -> ProviderId {
        (**self).provider()
    }

    fn otel_provider_name(&self) -> &'static str {
        (**self).otel_provider_name()
    }

    fn model(&self) -> &ModelId {
        (**self).model()
    }

    fn inference_params(&self) -> &InferenceParams {
        (**self).inference_params()
    }

    async fn send(&self, request: CompletionRequest) -> Result<ChatCompletionStream, ModelError> {
        (**self).send(request).await
    }
}

/// A provider client. Model instances are obtained from it via [`CompletionClient::model`].
pub trait CompletionClient: Send + Sync {
    /// The concrete model type this client produces.
    type Model: CompletionModel + 'static;

    /// The provider this client belongs to.
    fn provider(&self) -> ProviderId;

    /// Create a model instance for the given model name, with the given inference parameter
    /// defaults. These defaults are baked into the model instance and applied on every request
    /// unless overridden at request time.
    fn model(&self, model: ModelId, params: InferenceParams) -> Self::Model;
}

/// An erased version of [`CompletionClient`](crate::traits::CompletionClient) that can be used for dynamic dispatch.
pub trait ErasedCompletionClient: Send + Sync {
    fn provider(&self) -> ProviderId;

    fn model_erased(&self, model: ModelId, params: InferenceParams) -> Arc<dyn CompletionModel>;
}

impl<C: CompletionClient> ErasedCompletionClient for C {
    fn provider(&self) -> ProviderId {
        self.provider()
    }

    fn model_erased(&self, model: ModelId, params: InferenceParams) -> Arc<dyn CompletionModel> {
        Arc::new(self.model(model, params))
    }
}

/// Factory for constructing clients from typed configuration.
pub trait ClientFactory: Send + Sync {
    type Config: DeserializeOwned;
    type Client: CompletionClient + 'static;

    /// The provider this factory constructs clients for.
    fn provider(&self) -> ProviderId;

    /// Build a concrete client from typed config.
    fn build(&self, config: Self::Config) -> Result<Self::Client, ModelError>;
}

/// An erased version of [`ClientFactory`](crate::traits::ClientFactory) that can be used for dynamic dispatch.
pub trait ErasedClientFactory: Send + Sync {
    fn provider(&self) -> ProviderId;

    fn build_erased(&self, config: Value) -> Result<Arc<dyn ErasedCompletionClient>, ModelError>;
}

impl<F: ClientFactory> ErasedClientFactory for F {
    fn provider(&self) -> ProviderId {
        self.provider()
    }

    fn build_erased(&self, config: Value) -> Result<Arc<dyn ErasedCompletionClient>, ModelError> {
        self.build(from_value::<F::Config>(config).map_err(ModelError::Serialization)?)
            .map(|client| Arc::new(client) as Arc<dyn ErasedCompletionClient>)
    }
}

/// Builder for constructing a completion request with optional parameters in a fluent style.
pub struct CompletionRequestBuilder<'a> {
    model: &'a dyn CompletionModel,
    request: CompletionRequest,
}

impl<'a> CompletionRequestBuilder<'a> {
    /// Set the tools available to the model for this request.
    pub fn tools(mut self, tools: impl IntoIterator<Item = impl Into<ToolSpec>>) -> Self {
        self.request.tools = tools
            .into_iter()
            .map(Into::into)
            .collect();
        self
    }

    /// Optionally set the tools available to the model for this request.
    pub fn maybe_tools(
        mut self,
        tools: Option<impl IntoIterator<Item = impl Into<ToolSpec>>>,
    ) -> Self {
        if let Some(tools) = tools {
            self.request.tools = tools
                .into_iter()
                .map(Into::into)
                .collect();
        }
        self
    }

    /// Set the maximum number of tokens to generate.
    pub fn max_tokens(mut self, max_tokens: u64) -> Self {
        self.request.max_tokens = Some(max_tokens);
        self
    }

    /// Optionally set the maximum number of tokens to generate.
    pub fn maybe_max_tokens(mut self, max_tokens: Option<u64>) -> Self {
        if let Some(max_tokens) = max_tokens {
            self.request.max_tokens = Some(max_tokens);
        }
        self
    }

    /// Set the sampling temperature.
    pub fn temperature(mut self, temperature: impl Into<f64>) -> Self {
        self.request.temperature = Some(temperature.into());
        self
    }

    /// Optionally set the sampling temperature.
    pub fn maybe_temperature(mut self, temperature: Option<impl Into<f64>>) -> Self {
        if let Some(temperature) = temperature {
            self.request.temperature = Some(temperature.into());
        }
        self
    }

    /// Set the nucleus sampling threshold.
    pub fn top_p(mut self, top_p: impl Into<f64>) -> Self {
        self.request.top_p = Some(top_p.into());
        self
    }

    /// Optionally set the nucleus sampling threshold.
    pub fn maybe_top_p(mut self, top_p: Option<impl Into<f64>>) -> Self {
        if let Some(top_p) = top_p {
            self.request.top_p = Some(top_p.into());
        }
        self
    }

    /// Set the top-k sampling limit.
    pub fn top_k(mut self, top_k: u32) -> Self {
        self.request.top_k = Some(top_k);
        self
    }

    /// Optionally set the top-k sampling limit.
    pub fn maybe_top_k(mut self, top_k: Option<u32>) -> Self {
        if let Some(top_k) = top_k {
            self.request.top_k = Some(top_k);
        }
        self
    }

    /// Set sequences at which the model will stop generating.
    pub fn stop_sequences(
        mut self,
        stop_sequences: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.request.stop_sequences = Some(
            stop_sequences
                .into_iter()
                .map(Into::into)
                .collect(),
        );
        self
    }

    /// Optionally set sequences at which the model will stop generating.
    pub fn maybe_stop_sequences(
        mut self,
        stop_sequences: Option<impl IntoIterator<Item = impl Into<String>>>,
    ) -> Self {
        if let Some(stop_sequences) = stop_sequences {
            self.request.stop_sequences = Some(
                stop_sequences
                    .into_iter()
                    .map(Into::into)
                    .collect(),
            );
        }
        self
    }

    /// Set the frequency penalty.
    pub fn frequency_penalty(mut self, frequency_penalty: impl Into<f64>) -> Self {
        self.request.frequency_penalty = Some(frequency_penalty.into());
        self
    }

    /// Optionally set the frequency penalty.
    pub fn maybe_frequency_penalty(mut self, frequency_penalty: Option<impl Into<f64>>) -> Self {
        if let Some(frequency_penalty) = frequency_penalty {
            self.request.frequency_penalty = Some(frequency_penalty.into());
        }
        self
    }

    /// Set the presence penalty.
    pub fn presence_penalty(mut self, presence_penalty: impl Into<f64>) -> Self {
        self.request.presence_penalty = Some(presence_penalty.into());
        self
    }

    /// Optionally set the presence penalty.
    pub fn maybe_presence_penalty(mut self, presence_penalty: Option<impl Into<f64>>) -> Self {
        if let Some(presence_penalty) = presence_penalty {
            self.request.presence_penalty = Some(presence_penalty.into());
        }
        self
    }

    /// Set the seed for deterministic sampling.
    pub fn seed(mut self, seed: u64) -> Self {
        self.request.seed = Some(seed);
        self
    }

    /// Optionally set the seed for deterministic sampling.
    pub fn maybe_seed(mut self, seed: Option<u64>) -> Self {
        if let Some(seed) = seed {
            self.request.seed = Some(seed);
        }
        self
    }

    /// Set provider-specific parameters. These are serialized and forwarded to
    /// the provider implementation, which merges them on top of any config-level
    /// defaults. Request-level values take precedence.
    pub fn provider_params(mut self, params: impl Serialize) -> Self {
        self.request.provider_params = to_value(params).ok();
        self
    }

    /// Optionally set provider-specific parameters.
    pub fn maybe_provider_params(mut self, params: Option<impl Serialize>) -> Self {
        if let Some(params) = params {
            self.request.provider_params = to_value(params).ok();
        }
        self
    }

    /// Send the request and return a streaming response.
    pub async fn send(self) -> Result<ChatCompletionStream, ModelError> {
        self.model.send(self.request).await
    }
}

/// Extension trait for completion models, providing a convenient entry point for building requests.
pub trait CompletionModelExt {
    /// Begin building a completion request for the given messages.
    fn request<I, M>(&self, messages: I) -> CompletionRequestBuilder<'_>
    where
        I: IntoIterator<Item = M>,
        M: Into<ChatMessage>;

    /// Layer a [`CompletionMiddleware`] over this model, returning an
    /// [`Intercepted`] that is itself a [`CompletionModel`] and so composes
    /// with further layers.
    fn layer<L>(self, middleware: L) -> Intercepted<Self, L>
    where
        Self: Sized,
        L: CompletionMiddleware,
    {
        Intercepted::new(self, middleware)
    }

    /// Optionally construct and layer a [`CompletionMiddleware`] over this
    /// model while preserving a single concrete return type.
    fn layer_with<T, L>(
        self,
        value: Option<T>,
        build: impl FnOnce(T) -> L,
    ) -> Intercepted<Self, Option<L>>
    where
        Self: Sized,
        L: CompletionMiddleware,
    {
        self.layer(value.map(build))
    }
}

impl<T> CompletionModelExt for T
where
    T: CompletionModel,
{
    fn request<I, M>(&self, messages: I) -> CompletionRequestBuilder<'_>
    where
        I: IntoIterator<Item = M>,
        M: Into<ChatMessage>,
    {
        CompletionRequestBuilder {
            model: self,
            request: CompletionRequest::new(
                messages
                    .into_iter()
                    .map(Into::into)
                    .collect(),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use futures::stream;
    use std::time::Duration;

    use crate::{
        middleware::timeout::Timeout,
        types::{
            identity::{ModelId, ProviderId},
            inference::InferenceParams,
            stream::CompletionStreamEvent,
        },
    };

    struct StubModel {
        model_id: ModelId,
        params: InferenceParams,
    }

    #[async_trait]
    impl CompletionModel for StubModel {
        fn provider(&self) -> ProviderId {
            "stub".into()
        }

        fn otel_provider_name(&self) -> &'static str {
            "stub"
        }

        fn model(&self) -> &ModelId {
            &self.model_id
        }

        fn inference_params(&self) -> &InferenceParams {
            &self.params
        }

        async fn send(
            &self,
            _request: CompletionRequest,
        ) -> Result<ChatCompletionStream, ModelError> {
            Ok(ChatCompletionStream::new(stream::empty::<
                Result<CompletionStreamEvent, ModelError>,
            >()))
        }
    }

    #[test]
    fn repeated_optional_layers_remain_a_completion_model() {
        assert_eq!(
            StubModel {
                model_id: "test-model".into(),
                params: InferenceParams::default(),
            }
            .layer_with(Some(Duration::from_secs(1)), Timeout::new)
            .layer_with(None::<Duration>, Timeout::new)
            .otel_provider_name(),
            "stub",
        );
    }
}
