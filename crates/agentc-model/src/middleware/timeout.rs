// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use std::time::Duration;
use tokio::time::timeout;

use crate::{
    errors::ModelError, middleware::CompletionMiddleware, stream::ChatCompletionStream,
    traits::CompletionModel, types::request::CompletionRequest,
};

/// A [`CompletionMiddleware`] that bounds the request handshake with a timeout.
/// Only the handshake — the `send(...).await` that opens the stream — is bounded;
/// the streaming that follows is left to run for as long as it needs.
pub struct Timeout {
    handshake: Duration,
}

impl Timeout {
    pub fn new(handshake: Duration) -> Self {
        Self { handshake }
    }
}

#[async_trait]
impl CompletionMiddleware for Timeout {
    async fn send(
        &self,
        next: &dyn CompletionModel,
        request: CompletionRequest,
    ) -> Result<ChatCompletionStream, ModelError> {
        match timeout(self.handshake, next.send(request)).await {
            Ok(result) => result,
            Err(_) => Err(ModelError::timeout(next.provider(), self.handshake)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use async_trait::async_trait;
    use futures::stream;
    use tokio::time::sleep;

    use crate::types::{
        identity::{ModelId, ProviderId},
        inference::InferenceParams,
        stream::CompletionStreamEvent,
    };

    /// A stub model whose handshake sleeps before ever returning a stream.
    struct SleepingModel {
        model_id: ModelId,
        params: InferenceParams,
        delay: Duration,
    }

    #[async_trait]
    impl CompletionModel for SleepingModel {
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
            sleep(self.delay).await;

            Ok(ChatCompletionStream::new(stream::empty::<
                Result<CompletionStreamEvent, ModelError>,
            >()))
        }
    }

    #[tokio::test]
    async fn bounds_a_slow_handshake() {
        let model = SleepingModel {
            model_id: "test-model".into(),
            params: InferenceParams::default(),
            delay: Duration::from_secs(10),
        };

        let Err(error) = Timeout::new(Duration::from_millis(20))
            .send(&model, CompletionRequest::new(vec![]))
            .await
        else {
            panic!("handshake should time out");
        };

        assert!(matches!(error, ModelError::Timeout { .. }));
        assert!(error.is_transient());
    }
}
