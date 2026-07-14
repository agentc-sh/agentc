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

#[async_trait]
impl<L> CompletionMiddleware for Option<L>
where
    L: CompletionMiddleware,
{
    async fn send(
        &self,
        next: &dyn CompletionModel,
        request: CompletionRequest,
    ) -> Result<ChatCompletionStream, ModelError> {
        match self {
            Some(middleware) => middleware.send(next, request).await,
            None => next.send(request).await,
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    use futures::stream;
    use std::sync::atomic::{AtomicU32, Ordering};

    use crate::types::{
        identity::{ModelId, ProviderId},
        inference::InferenceParams,
        stream::CompletionStreamEvent,
    };

    struct CountingModel {
        model_id: ModelId,
        params: InferenceParams,
        calls: AtomicU32,
    }

    impl CountingModel {
        fn new() -> Self {
            Self {
                model_id: "test-model".into(),
                params: InferenceParams::default(),
                calls: AtomicU32::new(0),
            }
        }
    }

    #[async_trait]
    impl CompletionModel for CountingModel {
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
            self.calls
                .fetch_add(1, Ordering::SeqCst);

            Ok(ChatCompletionStream::new(stream::empty::<
                Result<CompletionStreamEvent, ModelError>,
            >()))
        }
    }

    struct CountingMiddleware {
        calls: AtomicU32,
    }

    #[async_trait]
    impl CompletionMiddleware for CountingMiddleware {
        async fn send(
            &self,
            next: &dyn CompletionModel,
            request: CompletionRequest,
        ) -> Result<ChatCompletionStream, ModelError> {
            self.calls
                .fetch_add(1, Ordering::SeqCst);

            next.send(request).await
        }
    }

    #[tokio::test]
    async fn optional_middleware_invokes_some() {
        let model = CountingModel::new();
        let middleware = Some(CountingMiddleware { calls: AtomicU32::new(0) });

        let result = middleware
            .send(&model, CompletionRequest::new(vec![]))
            .await;

        assert!(result.is_ok());
        assert_eq!(
            middleware
                .unwrap()
                .calls
                .load(Ordering::SeqCst),
            1
        );
        assert_eq!(model.calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn optional_middleware_delegates_none() {
        let model = CountingModel::new();

        let result = Option::<CountingMiddleware>::None
            .send(&model, CompletionRequest::new(vec![]))
            .await;

        assert!(result.is_ok());
        assert_eq!(model.calls.load(Ordering::SeqCst), 1);
    }
}
