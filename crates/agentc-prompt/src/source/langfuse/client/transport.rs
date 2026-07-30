// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::{
    cmp,
    time::{Duration, Instant, SystemTime},
};

use reqwest::{
    Method, StatusCode, Url,
    header::{AGE, CACHE_CONTROL, DATE, ETAG, EXPIRES, HeaderMap},
};

use super::{
    error::LangfuseError,
    types::{Prompt, PromptSelector},
    wire::WirePrompt,
};

const MAX_ERROR_BODY_BYTES: usize = 8 * 1024;

#[derive(Clone)]
pub(super) struct HttpTransport {
    base_url: Url,
    client: reqwest::Client,
    public_key: String,
    secret_key: String,
}

pub(super) struct FetchedPrompt {
    pub prompt: Prompt,
    pub fetched_at: Instant,
    pub metadata: ResponseMetadata,
}

#[derive(Clone)]
pub(super) struct ResponseMetadata {
    pub freshness: Option<Duration>,
    pub no_store: bool,
    pub no_cache: bool,
    pub etag: Option<String>,
}

impl HttpTransport {
    pub fn new(
        base_url: impl AsRef<str>,
        public_key: String,
        secret_key: String,
        fetch_timeout: Duration,
        max_retries: u32,
    ) -> Result<Self, LangfuseError> {
        let base_url = Self::normalize_base_url(base_url.as_ref())?;
        let host = base_url
            .host_str()
            .expect("validated Langfuse base URL should have a host")
            .to_string();

        Ok(Self {
            base_url,
            client: reqwest::Client::builder()
                .timeout(fetch_timeout)
                .retry(
                    reqwest::retry::for_host(host)
                        .max_retries_per_request(max_retries)
                        .classify_fn(|attempt| {
                            if attempt.method() != Method::GET {
                                return attempt.success();
                            }

                            match attempt.status() {
                                Some(StatusCode::TOO_MANY_REQUESTS) => attempt.retryable(),
                                Some(status) if status.is_server_error() => attempt.retryable(),
                                Some(_) => attempt.success(),
                                None if attempt.error().is_some() => attempt.retryable(),
                                None => attempt.success(),
                            }
                        }),
                )
                .build()
                .map_err(LangfuseError::request)?,
            public_key,
            secret_key,
        })
    }

    pub async fn fetch(
        &self,
        name: &str,
        selector: &PromptSelector,
    ) -> Result<FetchedPrompt, LangfuseError> {
        let mut request = self
            .client
            .get(self.prompt_url(name)?)
            .basic_auth(&self.public_key, Some(&self.secret_key));

        request = match selector {
            PromptSelector::Default => request,
            PromptSelector::Label(label) => request.query(&[("label", label)]),
            PromptSelector::Version(version) => request.query(&[("version", version)]),
        };

        let response = request
            .send()
            .await
            .map_err(LangfuseError::request)?;
        let fetched_at = Instant::now();

        if !response.status().is_success() {
            return Err(self.response_error(response).await);
        }

        let metadata = ResponseMetadata::from_headers(
            response.headers(),
            SystemTime::now(),
        );

        Ok(FetchedPrompt {
            prompt: response
                .json::<WirePrompt>()
                .await
                .map(Prompt::from)
                .map_err(LangfuseError::decode)?,
            fetched_at,
            metadata,
        })
    }

    fn normalize_base_url(base_url: &str) -> Result<Url, LangfuseError> {
        let mut url = Url::parse(base_url)
            .map_err(|error| LangfuseError::configuration(error.to_string()))?;

        if !matches!(url.scheme(), "http" | "https") {
            return Err(LangfuseError::configuration(
                "base URL must use HTTP or HTTPS",
            ));
        }

        if url.host_str().is_none() {
            return Err(LangfuseError::configuration(
                "base URL must include a host",
            ));
        }

        if !url.username().is_empty() || url.password().is_some() {
            return Err(LangfuseError::configuration(
                "base URL must not include credentials",
            ));
        }

        url.set_query(None);
        url.set_fragment(None);
        let normalized_path = url
            .path()
            .trim_end_matches('/')
            .to_string();
        url.set_path(&normalized_path);

        Ok(url)
    }

    fn prompt_url(&self, name: &str) -> Result<Url, LangfuseError> {
        let mut url = self.base_url.clone();

        url.path_segments_mut()
            .map_err(|_| LangfuseError::configuration("base URL cannot be a base"))?
            .pop_if_empty()
            .extend(["api", "public", "v2", "prompts"])
            .push(name);

        Ok(url)
    }

    async fn response_error(&self, mut response: reqwest::Response) -> LangfuseError {
        let status = response.status().as_u16();
        let mut body = Vec::new();

        loop {
            match response.chunk().await {
                Ok(Some(chunk)) => {
                    body.extend_from_slice(
                        &chunk[..cmp::min(
                            chunk.len(),
                            MAX_ERROR_BODY_BYTES.saturating_sub(body.len()),
                        )],
                    );

                    if body.len() == MAX_ERROR_BODY_BYTES {
                        break;
                    }
                }
                Ok(None) => break,
                Err(error) => return LangfuseError::request(error),
            }
        }

        LangfuseError::response(
            status,
            self.sanitize_error_body(&body),
        )
    }

    fn sanitize_error_body(&self, body: &[u8]) -> String {
        let mut message = String::from_utf8_lossy(body)
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .replace(&self.public_key, "[redacted]")
            .replace(&self.secret_key, "[redacted]");

        if message.len() > MAX_ERROR_BODY_BYTES {
            let mut end = MAX_ERROR_BODY_BYTES;

            while !message.is_char_boundary(end) {
                end -= 1;
            }

            message.truncate(end);
        }

        message
    }
}

impl ResponseMetadata {
    fn from_headers(headers: &HeaderMap, received_at: SystemTime) -> Self {
        let mut metadata = Self {
            freshness: None,
            no_store: false,
            no_cache: false,
            etag: headers
                .get(ETAG)
                .and_then(|value| value.to_str().ok())
                .map(str::to_string),
        };

        metadata.apply_cache_control(headers);
        metadata.apply_expires(headers, received_at);

        metadata
    }

    fn apply_cache_control(&mut self, headers: &HeaderMap) {
        for directive in headers
            .get_all(CACHE_CONTROL)
            .iter()
            .filter_map(|value| value.to_str().ok())
            .flat_map(|value| value.split(','))
            .map(str::trim)
        {
            if directive.eq_ignore_ascii_case("no-store") {
                self.no_store = true;
            } else if directive.eq_ignore_ascii_case("no-cache") {
                self.no_cache = true;
            } else if let Some((name, value)) = directive.split_once('=')
                && name.trim().eq_ignore_ascii_case("max-age")
                && let Ok(seconds) = value
                    .trim()
                    .trim_matches('"')
                    .parse::<u64>()
            {
                self.freshness = Some(
                    self.freshness
                        .map(|current| current.min(Duration::from_secs(seconds)))
                        .unwrap_or(Duration::from_secs(seconds)),
                );
            }
        }

        if let Some(age) = headers
            .get(AGE)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok())
        {
            self.freshness = self
                .freshness
                .map(|freshness| freshness.saturating_sub(Duration::from_secs(age)));
        }
    }

    fn apply_expires(&mut self, headers: &HeaderMap, received_at: SystemTime) {
        let Some(expires) = headers
            .get(EXPIRES)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| httpdate::parse_http_date(value).ok())
        else {
            return;
        };
        let reference = headers
            .get(DATE)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| httpdate::parse_http_date(value).ok())
            .unwrap_or(received_at);
        let freshness = expires
            .duration_since(reference)
            .unwrap_or(Duration::ZERO);

        self.freshness = Some(
            self.freshness
                .map(|current| current.min(freshness))
                .unwrap_or(freshness),
        );
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
        matchers::{
            header, method, path, query_param, query_param_is_missing,
        },
    };

    use super::*;

    struct TransportFixture;

    impl TransportFixture {
        async fn transport() -> (MockServer, HttpTransport) {
            let server = MockServer::start().await;
            let transport = Self::transport_for(&server, Duration::from_secs(5), 0);

            (server, transport)
        }

        fn transport_for(
            server: &MockServer,
            timeout: Duration,
            retries: u32,
        ) -> HttpTransport {
            HttpTransport::new(
                server.uri(),
                "public".to_string(),
                "secret".to_string(),
                timeout,
                retries,
            )
            .expect("transport should build")
        }

        fn text_prompt() -> serde_json::Value {
            json!({
                "type": "text",
                "name": "assistant",
                "version": 3,
                "config": {},
                "labels": ["production"],
                "tags": [],
                "commitMessage": null,
                "resolutionGraph": null,
                "prompt": "You are {{ agent_name }}.",
            })
        }

        fn chat_prompt() -> serde_json::Value {
            json!({
                "type": "chat",
                "name": "assistant",
                "version": 4,
                "config": {},
                "labels": ["staging"],
                "tags": [],
                "commitMessage": null,
                "resolutionGraph": null,
                "prompt": [
                    {
                        "type": "chatmessage",
                        "role": "system",
                        "content": "Be concise.",
                    },
                ],
            })
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
    async fn default_request_uses_basic_auth_without_selector_query() {
        let (server, transport) = TransportFixture::transport().await;

        Mock::given(method("GET"))
            .and(path("/api/public/v2/prompts/assistant"))
            .and(header(
                "authorization",
                "Basic cHVibGljOnNlY3JldA==",
            ))
            .and(query_param_is_missing("label"))
            .and(query_param_is_missing("version"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(TransportFixture::text_prompt()),
            )
            .mount(&server)
            .await;

        assert_eq!(
            transport
                .fetch("assistant", &PromptSelector::Default)
                .await
                .expect("prompt should load")
                .prompt
                .version(),
            3,
        );
    }

    #[tokio::test]
    async fn selectors_use_their_expected_query_parameters() {
        let (server, transport) = TransportFixture::transport().await;

        Mock::given(method("GET"))
            .and(path("/api/public/v2/prompts/assistant"))
            .and(query_param("label", "staging"))
            .and(query_param_is_missing("version"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(TransportFixture::chat_prompt()),
            )
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/api/public/v2/prompts/assistant"))
            .and(query_param("version", "3"))
            .and(query_param_is_missing("label"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(TransportFixture::text_prompt()),
            )
            .expect(1)
            .mount(&server)
            .await;

        transport
            .fetch(
                "assistant",
                &PromptSelector::Label("staging".to_string()),
            )
            .await
            .expect("label prompt should load");
        transport
            .fetch("assistant", &PromptSelector::Version(3))
            .await
            .expect("version prompt should load");
    }

    #[tokio::test]
    async fn prompt_folder_name_is_encoded_as_one_path_segment() {
        let (server, transport) = TransportFixture::transport().await;

        Mock::given(method("GET"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(TransportFixture::text_prompt()),
            )
            .mount(&server)
            .await;

        transport
            .fetch("support/assistant", &PromptSelector::Default)
            .await
            .expect("folder prompt should load");

        assert_eq!(
            server
                .received_requests()
                .await
                .expect("request recording should be enabled")[0]
                .url
                .path(),
            "/api/public/v2/prompts/support%2Fassistant",
        );
    }

    #[tokio::test]
    async fn response_decoding_preserves_text_and_chat_variants() {
        let (server, transport) = TransportFixture::transport().await;

        Mock::given(path("/api/public/v2/prompts/text"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(TransportFixture::text_prompt()),
            )
            .mount(&server)
            .await;
        Mock::given(path("/api/public/v2/prompts/chat"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(TransportFixture::chat_prompt()),
            )
            .mount(&server)
            .await;

        assert!(matches!(
            transport
                .fetch("text", &PromptSelector::Default)
                .await
                .expect("text prompt should load")
                .prompt,
            Prompt::Text(_)
        ));
        assert!(matches!(
            transport
                .fetch("chat", &PromptSelector::Default)
                .await
                .expect("chat prompt should load")
                .prompt,
            Prompt::Chat(_)
        ));
    }

    #[tokio::test]
    async fn non_success_context_is_bounded_and_redacted() {
        let (server, transport) = TransportFixture::transport().await;

        Mock::given(method("GET"))
            .respond_with(
                ResponseTemplate::new(400)
                    .set_body_string(format!("public secret {}", "x".repeat(9_000))),
            )
            .mount(&server)
            .await;

        assert!(matches!(
            transport
                .fetch("assistant", &PromptSelector::Default)
                .await,
            Err(LangfuseError::Response { status: 400, message })
                if message.len() <= MAX_ERROR_BODY_BYTES
                    && !message.contains("public")
                    && !message.contains("secret")
        ));
    }

    #[tokio::test]
    async fn transient_response_is_retried() {
        let server = MockServer::start().await;
        let calls = Arc::new(AtomicUsize::new(0));

        Mock::given(method("GET"))
            .respond_with(SequenceResponder {
                calls: calls.clone(),
                first: ResponseTemplate::new(500),
                remaining: ResponseTemplate::new(200)
                    .set_body_json(TransportFixture::text_prompt()),
            })
            .mount(&server)
            .await;

        TransportFixture::transport_for(&server, Duration::from_secs(5), 1)
            .fetch("assistant", &PromptSelector::Default)
            .await
            .expect("transient response should recover");

        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn terminal_client_response_is_not_retried() {
        let server = MockServer::start().await;
        let calls = Arc::new(AtomicUsize::new(0));

        Mock::given(method("GET"))
            .respond_with(SequenceResponder {
                calls: calls.clone(),
                first: ResponseTemplate::new(401),
                remaining: ResponseTemplate::new(200)
                    .set_body_json(TransportFixture::text_prompt()),
            })
            .mount(&server)
            .await;

        assert!(matches!(
            TransportFixture::transport_for(&server, Duration::from_secs(5), 2)
                .fetch("assistant", &PromptSelector::Default)
                .await,
            Err(LangfuseError::Response { status: 401, .. })
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn request_obeys_per_attempt_timeout() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_delay(Duration::from_millis(100))
                    .set_body_json(TransportFixture::text_prompt()),
            )
            .mount(&server)
            .await;

        assert!(matches!(
            TransportFixture::transport_for(
                &server,
                Duration::from_millis(10),
                0,
            )
            .fetch("assistant", &PromptSelector::Default)
            .await,
            Err(LangfuseError::Request { .. })
        ));
    }
}
