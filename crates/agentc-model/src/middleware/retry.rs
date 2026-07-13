// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::sleep;

use crate::{
    errors::ModelError,
    middleware::CompletionMiddleware,
    stream::ChatCompletionStream,
    traits::CompletionModel,
    types::request::CompletionRequest,
};

/// Policy governing how a [`Retry`] middleware re-attempts a failed handshake.
pub struct RetryPolicy {
    /// Maximum number of handshake attempts, including the first.
    pub max_attempts: u32,
    /// Backoff before the second attempt; doubled on each subsequent attempt.
    pub initial_backoff: Duration,
    /// Upper bound on any single backoff, applied before jitter.
    pub max_backoff: Duration,
}

impl RetryPolicy {
    /// The backoff to wait after the given (1-based) attempt number, capped at
    /// [`max_backoff`](RetryPolicy::max_backoff) and offset by jitter.
    pub fn backoff_for(&self, attempt: u32) -> Duration {
        let capped = self
            .initial_backoff
            .saturating_mul(2u32.saturating_pow(attempt.saturating_sub(1)))
            .min(self.max_backoff);

        capped + Self::jitter(capped)
    }

    /// A small non-negative offset derived without a random-number dependency,
    /// so concurrent retriers do not synchronize their backoff.
    fn jitter(base: Duration) -> Duration {
        let fraction = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|since| since.subsec_nanos() % 1000)
            .unwrap_or(0);

        (base / 4).saturating_mul(fraction) / 1000
    }
}

/// A [`CompletionMiddleware`] that retries the request handshake while the
/// returned [`ModelError`] is [`is_transient`](ModelError::is_transient) and
/// attempts remain, backing off between attempts and honoring a
/// [`retry_after`](ModelError::retry_after) hint when present.
pub struct Retry {
    policy: RetryPolicy,
}

impl Retry {
    pub fn new(policy: RetryPolicy) -> Self {
        Self { policy }
    }
}

#[async_trait]
impl CompletionMiddleware for Retry {
    async fn send(
        &self,
        next: &dyn CompletionModel,
        request: CompletionRequest,
    ) -> Result<ChatCompletionStream, ModelError> {
        let mut attempt = 1;

        loop {
            match next.send(request.clone()).await {
                Ok(stream) => return Ok(stream),
                Err(error) if error.is_transient() && attempt < self.policy.max_attempts => {
                    sleep(
                        error
                            .retry_after()
                            .unwrap_or_else(|| self.policy.backoff_for(attempt)),
                    )
                    .await;

                    attempt += 1;
                }
                Err(error) => return Err(error),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use async_trait::async_trait;
    use futures::stream;
    use std::sync::atomic::{AtomicU32, Ordering};

    use crate::types::{
        identity::{ModelId, ProviderId},
        inference::InferenceParams,
        stream::CompletionStreamEvent,
    };

    /// A stub model that fails its first `fail_until` handshakes, then succeeds.
    /// The failures are transient or not depending on `transient`.
    struct FlakyModel {
        model_id: ModelId,
        params: InferenceParams,
        calls: AtomicU32,
        fail_until: u32,
        transient: bool,
    }

    impl FlakyModel {
        fn new(fail_until: u32, transient: bool) -> Self {
            Self {
                model_id: "test-model".into(),
                params: InferenceParams::default(),
                calls: AtomicU32::new(0),
                fail_until,
                transient,
            }
        }
    }

    #[async_trait]
    impl CompletionModel for FlakyModel {
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
            let attempt = self.calls.fetch_add(1, Ordering::SeqCst) + 1;

            if attempt > self.fail_until {
                return Ok(ChatCompletionStream::new(stream::empty::<
                    Result<CompletionStreamEvent, ModelError>,
                >()));
            }

            if self.transient {
                Err(ModelError::transient("stub", "temporary", None, None::<std::io::Error>))
            } else {
                Err(ModelError::invalid_request("permanent"))
            }
        }
    }

    fn policy() -> RetryPolicy {
        RetryPolicy {
            max_attempts: 3,
            initial_backoff: Duration::ZERO,
            max_backoff: Duration::ZERO,
        }
    }

    #[tokio::test]
    async fn retries_transient_then_succeeds() {
        let model = FlakyModel::new(1, true);

        let result = Retry::new(policy())
            .send(&model, CompletionRequest::new(vec![]))
            .await;

        assert!(result.is_ok());
        assert_eq!(model.calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn does_not_retry_non_transient() {
        let model = FlakyModel::new(u32::MAX, false);

        let Err(error) = Retry::new(policy())
            .send(&model, CompletionRequest::new(vec![]))
            .await
        else {
            panic!("non-transient error should not be retried");
        };

        assert!(matches!(error, ModelError::InvalidRequest { .. }));
        assert_eq!(model.calls.load(Ordering::SeqCst), 1);
    }
}
