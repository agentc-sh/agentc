// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;

use crate::{
    errors::PromptError,
    source::{
        PromptSource,
        langfuse::{
            LangfusePromptSourceBuilder,
            client::{GetPromptRequest, LangfuseClient},
        },
    },
    template::PromptTemplate,
};

/// A prompt source backed by Langfuse Prompt Management.
pub struct LangfusePromptSource {
    pub(super) client: LangfuseClient,
    pub(super) prompt_name: String,
    pub(super) request: GetPromptRequest,
}

impl LangfusePromptSource {
    pub fn builder() -> LangfusePromptSourceBuilder {
        LangfusePromptSourceBuilder::new()
    }
}

#[async_trait]
impl PromptSource for LangfusePromptSource {
    async fn load(&self) -> Result<PromptTemplate, PromptError> {
        self.client
            .prompts()
            .get(&self.prompt_name, self.request.clone())
            .await
            .map_err(|error| {
                PromptError::sourced_source(
                    format!("failed to load Langfuse prompt `{}`", self.prompt_name,),
                    error,
                )
            })?
            .try_into()
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error as _;

    use wiremock::{Mock, MockServer, ResponseTemplate, matchers::method};

    use super::*;
    use crate::source::langfuse::client::LangfuseError;

    #[tokio::test]
    async fn load_preserves_client_error_in_source_chain() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&server)
            .await;

        let error = LangfusePromptSource::builder()
            .client(
                LangfuseClient::builder()
                    .public_key("public")
                    .secret_key("secret")
                    .base_url(server.uri())
                    .max_retries(0)
                    .build()
                    .expect("client should build"),
            )
            .prompt_name("assistant")
            .build()
            .expect("source should build")
            .load()
            .await
            .expect_err("load should fail");

        assert!(error.to_string().contains("assistant"));
        assert!(error.source().is_some_and(|source| {
            source
                .downcast_ref::<LangfuseError>()
                .is_some()
        }));
    }
}
