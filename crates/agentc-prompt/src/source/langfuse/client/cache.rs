// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::time::Duration;

use moka::{
    future::Cache,
    ops::compute::{CompResult, Op},
    policy::Expiry,
};

use super::{
    error::LangfuseError,
    transport::{FetchedPrompt, HttpTransport},
    types::{GetPromptRequest, Prompt, PromptCacheMode, PromptSelector},
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct PromptCacheKey {
    name: String,
    selector: PromptSelector,
}

#[derive(Clone)]
pub(super) struct CachedPrompt {
    prompt: Prompt,
    fetched_at: std::time::Instant,
    provider_freshness: Option<Duration>,
    no_store: bool,
    no_cache: bool,
    #[allow(dead_code)]
    etag: Option<String>,
}

pub(super) struct PromptExpiry;

#[derive(Clone)]
pub(super) struct PromptStore {
    transport: HttpTransport,
    cache: Cache<PromptCacheKey, CachedPrompt>,
    default_ttl: Duration,
}

impl PromptCacheKey {
    fn new(name: impl Into<String>, selector: PromptSelector) -> Self {
        Self { name: name.into(), selector }
    }
}

impl CachedPrompt {
    fn is_fresh(&self, selector: &PromptSelector, ttl: Duration) -> bool {
        if self.no_store || self.no_cache {
            return false;
        }

        if matches!(selector, PromptSelector::Version(_)) {
            return true;
        }

        self.fetched_at.elapsed()
            < self
                .provider_freshness
                .map(|freshness| freshness.min(ttl))
                .unwrap_or(ttl)
    }
}

impl From<FetchedPrompt> for CachedPrompt {
    fn from(value: FetchedPrompt) -> Self {
        Self {
            prompt: value.prompt,
            fetched_at: value.fetched_at,
            provider_freshness: value.metadata.freshness,
            no_store: value.metadata.no_store,
            no_cache: value.metadata.no_cache,
            etag: value.metadata.etag,
        }
    }
}

impl Expiry<PromptCacheKey, CachedPrompt> for PromptExpiry {
    fn expire_after_create(
        &self,
        key: &PromptCacheKey,
        value: &CachedPrompt,
        _created_at: std::time::Instant,
    ) -> Option<Duration> {
        Self::expiration(key, value)
    }

    fn expire_after_update(
        &self,
        key: &PromptCacheKey,
        value: &CachedPrompt,
        _updated_at: std::time::Instant,
        _duration_until_expiry: Option<Duration>,
    ) -> Option<Duration> {
        Self::expiration(key, value)
    }
}

impl PromptExpiry {
    fn expiration(key: &PromptCacheKey, value: &CachedPrompt) -> Option<Duration> {
        if value.no_store || value.no_cache {
            Some(Duration::ZERO)
        } else if matches!(key.selector, PromptSelector::Version(_)) {
            None
        } else {
            value.provider_freshness
        }
    }
}

impl PromptStore {
    pub fn new(transport: HttpTransport, default_ttl: Duration, capacity: u64) -> Self {
        Self {
            transport,
            cache: Cache::builder()
                .max_capacity(capacity)
                .expire_after(PromptExpiry)
                .build(),
            default_ttl,
        }
    }

    pub async fn get(
        &self,
        name: impl Into<String>,
        request: GetPromptRequest,
    ) -> Result<Prompt, LangfuseError> {
        let name = name.into();
        let ttl = match request.cache {
            PromptCacheMode::Disabled => {
                return self
                    .transport
                    .fetch(&name, &request.selector)
                    .await
                    .map(|fetched| fetched.prompt);
            }
            PromptCacheMode::Default if self.default_ttl.is_zero() => {
                return self
                    .transport
                    .fetch(&name, &request.selector)
                    .await
                    .map(|fetched| fetched.prompt);
            }
            PromptCacheMode::Default => self.default_ttl,
            PromptCacheMode::TimeToLive(ttl) => ttl,
        };
        let key = PromptCacheKey::new(name, request.selector);

        Self::prompt_from_result(
            self.cache
                .entry(key.clone())
                .and_try_compute_with(|entry| async move {
                    if entry
                        .as_ref()
                        .is_some_and(|entry| entry.value().is_fresh(&key.selector, ttl))
                    {
                        return Ok(Op::Nop);
                    }

                    self.transport
                        .fetch(&key.name, &key.selector)
                        .await
                        .map(CachedPrompt::from)
                        .map(Op::Put)
                })
                .await?,
        )
    }

    pub async fn invalidate(&self, name: impl Into<String>, selector: PromptSelector) {
        self.cache
            .invalidate(&PromptCacheKey::new(name, selector))
            .await;
    }

    pub async fn invalidate_name(&self, name: &str) {
        for key in self
            .cache
            .iter()
            .filter_map(|(key, _)| (key.name == name).then(|| key.as_ref().clone()))
            .collect::<Vec<_>>()
        {
            self.cache
                .invalidate(&key)
                .await;
        }
    }

    fn prompt_from_result(
        result: CompResult<PromptCacheKey, CachedPrompt>,
    ) -> Result<Prompt, LangfuseError> {
        match result {
            CompResult::Unchanged(entry)
            | CompResult::Inserted(entry)
            | CompResult::ReplacedWith(entry) => Ok(entry.value().prompt.clone()),
            CompResult::StillNone(_) | CompResult::Removed(_) => Err(LangfuseError::Cache),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    use serde_json::json;
    use wiremock::{
        Mock, MockServer, Request, Respond, ResponseTemplate,
        matchers::method,
    };

    use super::*;

    struct StoreFixture;

    impl StoreFixture {
        async fn store() -> (MockServer, PromptStore) {
            let server = MockServer::start().await;
            let store = Self::store_for(&server, Duration::from_secs(60));

            (server, store)
        }

        fn store_for(server: &MockServer, default_ttl: Duration) -> PromptStore {
            PromptStore::new(
                HttpTransport::new(
                    server.uri(),
                    "public".to_string(),
                    "secret".to_string(),
                    Duration::from_secs(5),
                    0,
                )
                .expect("transport should build"),
                default_ttl,
                128,
            )
        }

        fn response() -> ResponseTemplate {
            ResponseTemplate::new(200).set_body_json(json!({
                "type": "text",
                "name": "assistant",
                "version": 3,
                "config": {},
                "labels": ["production"],
                "tags": [],
                "commitMessage": null,
                "resolutionGraph": null,
                "prompt": "You are helpful.",
            }))
        }

        async fn request_count(server: &MockServer) -> usize {
            server
                .received_requests()
                .await
                .expect("request recording should be enabled")
                .len()
        }
    }

    struct SequenceResponder {
        calls: Arc<AtomicUsize>,
        first: ResponseTemplate,
        remaining: ResponseTemplate,
    }

    impl Respond for SequenceResponder {
        fn respond(&self, _request: &Request) -> ResponseTemplate {
            if self.calls.fetch_add(1, Ordering::SeqCst) == 0 {
                self.first.clone()
            } else {
                self.remaining.clone()
            }
        }
    }

    #[tokio::test]
    async fn repeated_fresh_loads_use_one_request() {
        let (server, store) = StoreFixture::store().await;

        Mock::given(method("GET"))
            .respond_with(StoreFixture::response())
            .mount(&server)
            .await;

        store
            .get("assistant", GetPromptRequest::new())
            .await
            .expect("first load should succeed");
        store
            .get("assistant", GetPromptRequest::new())
            .await
            .expect("second load should succeed");

        assert_eq!(StoreFixture::request_count(&server).await, 1);
    }

    #[tokio::test]
    async fn concurrent_cold_loads_coalesce() {
        let (server, store) = StoreFixture::store().await;

        Mock::given(method("GET"))
            .respond_with(
                StoreFixture::response()
                    .set_delay(Duration::from_millis(20)),
            )
            .mount(&server)
            .await;

        let (first, second) = tokio::join!(
            store.get("assistant", GetPromptRequest::new()),
            store.get("assistant", GetPromptRequest::new()),
        );

        first.expect("first load should succeed");
        second.expect("second load should succeed");
        assert_eq!(StoreFixture::request_count(&server).await, 1);
    }

    #[tokio::test]
    async fn disabled_cache_fetches_every_time() {
        let (server, store) = StoreFixture::store().await;

        Mock::given(method("GET"))
            .respond_with(StoreFixture::response())
            .mount(&server)
            .await;

        store
            .get(
                "assistant",
                GetPromptRequest::new()
                    .without_cache(),
            )
            .await
            .expect("first load should succeed");
        store
            .get(
                "assistant",
                GetPromptRequest::new()
                    .without_cache(),
            )
            .await
            .expect("second load should succeed");

        assert_eq!(StoreFixture::request_count(&server).await, 2);
    }

    #[tokio::test]
    async fn no_store_response_is_not_reused() {
        let (server, store) = StoreFixture::store().await;

        Mock::given(method("GET"))
            .respond_with(
                StoreFixture::response()
                    .append_header("cache-control", "no-store"),
            )
            .mount(&server)
            .await;

        store
            .get("assistant", GetPromptRequest::new())
            .await
            .expect("first load should succeed");
        store
            .get("assistant", GetPromptRequest::new())
            .await
            .expect("second load should succeed");

        assert_eq!(StoreFixture::request_count(&server).await, 2);
    }

    #[tokio::test]
    async fn failed_loads_are_not_cached() {
        let (server, store) = StoreFixture::store().await;

        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;

        assert!(
            store
                .get("assistant", GetPromptRequest::new())
                .await
                .is_err()
        );
        assert!(
            store
                .get("assistant", GetPromptRequest::new())
                .await
                .is_err()
        );
        assert_eq!(StoreFixture::request_count(&server).await, 2);
    }

    #[tokio::test]
    async fn version_selection_has_no_time_based_expiry() {
        let (server, store) = StoreFixture::store().await;

        Mock::given(method("GET"))
            .respond_with(
                StoreFixture::response()
                    .append_header("cache-control", "max-age=0"),
            )
            .mount(&server)
            .await;

        store
            .get(
                "assistant",
                GetPromptRequest::new()
                    .with_version(3)
                    .with_cache_ttl(Duration::from_nanos(1)),
            )
            .await
            .expect("first load should succeed");
        tokio::time::sleep(Duration::from_millis(5)).await;
        store
            .get(
                "assistant",
                GetPromptRequest::new()
                    .with_version(3)
                    .with_cache_ttl(Duration::from_nanos(1)),
            )
            .await
            .expect("second load should succeed");

        assert_eq!(StoreFixture::request_count(&server).await, 1);
    }

    #[tokio::test]
    async fn failed_stale_refresh_returns_error_instead_of_stale_prompt() {
        let server = MockServer::start().await;
        let store = StoreFixture::store_for(&server, Duration::from_millis(1));

        Mock::given(method("GET"))
            .respond_with(SequenceResponder {
                calls: Arc::new(AtomicUsize::new(0)),
                first: StoreFixture::response(),
                remaining: ResponseTemplate::new(500),
            })
            .mount(&server)
            .await;

        store
            .get("assistant", GetPromptRequest::new())
            .await
            .expect("first load should succeed");

        tokio::time::sleep(Duration::from_millis(5)).await;

        assert!(
            store
                .get("assistant", GetPromptRequest::new())
                .await
                .is_err()
        );
    }
}
