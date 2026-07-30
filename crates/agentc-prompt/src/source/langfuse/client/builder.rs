// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::time::Duration;

use super::{
    LangfuseClient,
    cache::PromptStore,
    error::LangfuseError,
    transport::HttpTransport,
};

pub const DEFAULT_BASE_URL: &str = "https://cloud.langfuse.com";
pub const DEFAULT_PROMPT_CACHE_CAPACITY: u64 = 128;
pub const DEFAULT_PROMPT_CACHE_TTL: Duration = Duration::from_secs(60);
pub const DEFAULT_FETCH_TIMEOUT: Duration = Duration::from_secs(5);
pub const DEFAULT_MAX_RETRIES: u32 = 2;

/// Builds a [`LangfuseClient`].
pub struct LangfuseClientBuilder {
    public_key: Option<String>,
    secret_key: Option<String>,
    base_url: String,
    fetch_timeout: Duration,
    max_retries: u32,
    prompt_cache_ttl: Duration,
    prompt_cache_capacity: u64,
}

impl LangfuseClientBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn public_key(mut self, public_key: impl Into<String>) -> Self {
        self.public_key = Some(public_key.into());
        self
    }

    pub fn secret_key(mut self, secret_key: impl Into<String>) -> Self {
        self.secret_key = Some(secret_key.into());
        self
    }

    pub fn base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    pub fn fetch_timeout(mut self, fetch_timeout: Duration) -> Self {
        self.fetch_timeout = fetch_timeout;
        self
    }

    pub fn max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    pub fn prompt_cache_ttl(mut self, prompt_cache_ttl: Duration) -> Self {
        self.prompt_cache_ttl = prompt_cache_ttl;
        self
    }

    pub fn prompt_cache_capacity(mut self, prompt_cache_capacity: u64) -> Self {
        self.prompt_cache_capacity = prompt_cache_capacity;
        self
    }

    pub fn build(self) -> Result<LangfuseClient, LangfuseError> {
        let public_key = self
            .public_key
            .filter(|value| !value.is_empty())
            .ok_or(LangfuseError::MissingField("public_key"))?;
        let secret_key = self
            .secret_key
            .filter(|value| !value.is_empty())
            .ok_or(LangfuseError::MissingField("secret_key"))?;

        Ok(
            LangfuseClient::new(
                PromptStore::new(
                    HttpTransport::new(
                        self.base_url,
                        public_key,
                        secret_key,
                        self.fetch_timeout,
                        self.max_retries,
                    )?,
                    self.prompt_cache_ttl,
                    self.prompt_cache_capacity,
                )
            )
        )
    }
}

impl Default for LangfuseClientBuilder {
    fn default() -> Self {
        Self {
            public_key: None,
            secret_key: None,
            base_url: DEFAULT_BASE_URL.to_string(),
            fetch_timeout: DEFAULT_FETCH_TIMEOUT,
            max_retries: DEFAULT_MAX_RETRIES,
            prompt_cache_ttl: DEFAULT_PROMPT_CACHE_TTL,
            prompt_cache_capacity: DEFAULT_PROMPT_CACHE_CAPACITY,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_requires_nonempty_credentials() {
        assert!(matches!(
            LangfuseClient::builder().build(),
            Err(LangfuseError::MissingField("public_key"))
        ));
        assert!(matches!(
            LangfuseClient::builder()
                .public_key("public")
                .secret_key("")
                .build(),
            Err(LangfuseError::MissingField("secret_key"))
        ));
    }

    #[test]
    fn builder_rejects_invalid_base_urls() {
        assert!(matches!(
            LangfuseClient::builder()
                .public_key("public")
                .secret_key("secret")
                .base_url("file:///tmp/langfuse")
                .build(),
            Err(LangfuseError::Configuration(_))
        ));
    }

    #[test]
    fn builder_errors_do_not_expose_credentials() {
        let Err(error) = LangfuseClient::builder()
            .public_key("sensitive-public")
            .secret_key("sensitive-secret")
            .base_url("https://sensitive-public:sensitive-secret@example.com")
            .build()
        else {
            panic!("embedded credentials should fail");
        };

        assert!(!error.to_string().contains("sensitive-public"));
        assert!(!error.to_string().contains("sensitive-secret"));
        assert!(!format!("{error:?}").contains("sensitive-public"));
        assert!(!format!("{error:?}").contains("sensitive-secret"));
    }
}
