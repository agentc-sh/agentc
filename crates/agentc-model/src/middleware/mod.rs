// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

pub mod retry;
pub mod timeout;

use async_trait::async_trait;

use crate::{
    errors::ModelError,
    stream::ChatCompletionStream,
    traits::CompletionModel,
    types::{
        identity::{ModelId, ProviderId},
        inference::InferenceParams,
        request::CompletionRequest,
    },
};

/// A behavior interposed around a model's handshake. Implementors customize only
/// [`send`](CompletionMiddleware::send); the surrounding [`Intercepted`] adapter
/// delegates every other part of the [`CompletionModel`] contract to the inner
/// model, so a middleware never reimplements the metadata accessors.
#[async_trait]
pub trait CompletionMiddleware: Send + Sync {
    /// Handle a request, delegating to `next` (the inner model) zero, one, or
    /// many times as the behavior requires.
    async fn send(
        &self,
        next: &dyn CompletionModel,
        request: CompletionRequest,
    ) -> Result<ChatCompletionStream, ModelError>;
}

/// Turns any [`CompletionMiddleware`] into a full [`CompletionModel`] by holding
/// an inner model and delegating every method except `send` to it. Constructed
/// via [`CompletionModelExt::layer`](crate::traits::CompletionModelExt::layer).
pub struct Intercepted<M, L> {
    inner: M,
    middleware: L,
}

impl<M, L> Intercepted<M, L> {
    pub fn new(inner: M, middleware: L) -> Self {
        Self { inner, middleware }
    }
}

#[async_trait]
impl<M, L> CompletionModel for Intercepted<M, L>
where
    M: CompletionModel,
    L: CompletionMiddleware,
{
    fn provider(&self) -> ProviderId {
        self.inner.provider()
    }

    fn otel_provider_name(&self) -> &'static str {
        self.inner.otel_provider_name()
    }

    fn model(&self) -> &ModelId {
        self.inner.model()
    }

    fn inference_params(&self) -> &InferenceParams {
        self.inner.inference_params()
    }

    async fn send(&self, request: CompletionRequest) -> Result<ChatCompletionStream, ModelError> {
        self.middleware
            .send(&self.inner, request)
            .await
    }
}
