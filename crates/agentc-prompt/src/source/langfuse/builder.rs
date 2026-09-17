// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::time::Duration;

use thiserror::Error;

use super::{
    LangfusePromptSource,
    client::{GetPromptRequest, LangfuseClient},
};

/// An invalid Langfuse prompt source configuration.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum LangfusePromptSourceConfigError {
    #[error("Langfuse prompt source requires a client")]
    MissingClient,

    #[error("Langfuse prompt source requires a prompt name")]
    MissingPromptName,

    #[error("Langfuse prompt source prompt name cannot be empty")]
    EmptyPromptName,

    #[error("Langfuse prompt source cannot set both label and version")]
    SelectorConflict,
}

/// Builds a [`LangfusePromptSource`].
#[derive(Default)]
pub struct LangfusePromptSourceBuilder {
    client: Option<LangfuseClient>,
    prompt_name: Option<String>,
    label: Option<String>,
    version: Option<u32>,
    cache_ttl: Option<Duration>,
}

impl LangfusePromptSourceBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn client(mut self, client: LangfuseClient) -> Self {
        self.client = Some(client);
        self
    }

    pub fn prompt_name(mut self, name: impl Into<String>) -> Self {
        self.prompt_name = Some(name.into());
        self
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn version(mut self, version: u32) -> Self {
        self.version = Some(version);
        self
    }

    pub fn cache_ttl(mut self, ttl: Duration) -> Self {
        self.cache_ttl = Some(ttl);
        self
    }

    pub fn build(self) -> Result<LangfusePromptSource, LangfusePromptSourceConfigError> {
        let client = self
            .client
            .ok_or(LangfusePromptSourceConfigError::MissingClient)?;
        let prompt_name = self
            .prompt_name
            .ok_or(LangfusePromptSourceConfigError::MissingPromptName)?;

        if prompt_name.is_empty() {
            return Err(LangfusePromptSourceConfigError::EmptyPromptName);
        }

        let request = match (self.label, self.version) {
            (Some(_), Some(_)) => {
                return Err(LangfusePromptSourceConfigError::SelectorConflict);
            }
            (Some(label), None) => GetPromptRequest::new().with_label(label),
            (None, Some(version)) => GetPromptRequest::new().with_version(version),
            (None, None) => GetPromptRequest::new(),
        };
        let request = match self.cache_ttl {
            Some(ttl) => request.with_cache_ttl(ttl),
            None => request,
        };

        Ok(LangfusePromptSource { client, prompt_name, request })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::langfuse::client::{PromptCacheMode, PromptSelector};

    struct BuilderFixture;

    impl BuilderFixture {
        fn client() -> LangfuseClient {
            LangfuseClient::builder()
                .public_key("public")
                .secret_key("secret")
                .base_url("http://localhost")
                .build()
                .expect("client should build")
        }

        fn builder() -> LangfusePromptSourceBuilder {
            LangfusePromptSource::builder()
                .client(Self::client())
                .prompt_name("assistant")
        }
    }

    #[test]
    fn builder_requires_client_and_nonempty_prompt_name() {
        assert!(matches!(
            LangfusePromptSource::builder()
                .prompt_name("assistant")
                .build(),
            Err(LangfusePromptSourceConfigError::MissingClient)
        ));
        assert!(matches!(
            LangfusePromptSource::builder()
                .client(BuilderFixture::client())
                .build(),
            Err(LangfusePromptSourceConfigError::MissingPromptName)
        ));
        assert!(matches!(
            LangfusePromptSource::builder()
                .client(BuilderFixture::client())
                .prompt_name("")
                .build(),
            Err(LangfusePromptSourceConfigError::EmptyPromptName)
        ));
    }

    #[test]
    fn builder_rejects_simultaneous_selectors() {
        assert!(matches!(
            BuilderFixture::builder()
                .label("staging")
                .version(7)
                .build(),
            Err(LangfusePromptSourceConfigError::SelectorConflict)
        ));
    }

    #[test]
    fn builder_configures_label_or_version() {
        assert_eq!(
            BuilderFixture::builder()
                .label("staging")
                .build()
                .expect("label source should build")
                .request
                .selector,
            PromptSelector::Label("staging".to_string()),
        );
        assert_eq!(
            BuilderFixture::builder()
                .version(7)
                .build()
                .expect("version source should build")
                .request
                .selector,
            PromptSelector::Version(7),
        );
    }

    #[test]
    fn builder_configures_positive_and_zero_cache_ttl() {
        assert_eq!(
            BuilderFixture::builder()
                .cache_ttl(Duration::from_secs(30))
                .build()
                .expect("cached source should build")
                .request
                .cache,
            PromptCacheMode::TimeToLive(Duration::from_secs(30)),
        );
        assert_eq!(
            BuilderFixture::builder()
                .cache_ttl(Duration::ZERO)
                .build()
                .expect("uncached source should build")
                .request
                .cache,
            PromptCacheMode::Disabled,
        );
    }
}
